import {
  Chain,
  ChainJson,
  CoreDeployAddresses,
  DeployEnvironment,
  RustConfig,
  toChain,
} from '../chain';
import { CoreContracts } from './CoreContracts';
import { Deploy } from '../deploy';
import { BigNumberish } from '@ethersproject/bignumber';
import { readFileSync } from 'fs';
import { getVerificationInputFromDeploy } from '../verification/readDeployOutput';
import path from 'path';

type Address = string;

type Governor = {
  domain: number;
  address: Address;
};

export type CoreConfig = {
  environment: DeployEnvironment;
  updater: Address;
  recoveryTimelock: number;
  recoveryManager: Address;
  optimisticSeconds: number;
  watchers: string[];
  governor?: Governor;
  processGas: BigNumberish;
  reserveGas: BigNumberish;
};

export class CoreDeploy extends Deploy<CoreContracts> {
  config: CoreConfig;

  constructor(chain: Chain, config: CoreConfig, test: boolean = false) {
    super(chain, new CoreContracts(), test);
    this.config = config;
  }

  get contractOutput(): CoreDeployAddresses {
    let addresses: CoreDeployAddresses = {
      ...this.contracts.toObject(),
      recoveryManager: this.config.recoveryManager,
      updater: this.config.updater,
    };
    if (this.config.governor) {
      addresses.governor = {
        address: this.config.governor.address,
        domain: this.chain.domain,
      };
    }
    return addresses;
  }

  get ubcAddress(): Address | undefined {
    return this.contracts.upgradeBeaconController?.address;
  }

  async governor(): Promise<Address> {
    return this.config.governor?.address ?? (await this.deployer.getAddress());
  }

  static parseCoreConfig(config: ChainJson & CoreConfig): [Chain, CoreConfig] {
    const chain = toChain(config);
    return [
      chain,
      {
        environment: config.environment,
        updater: config.updater,
        watchers: config.watchers ?? [],
        recoveryManager: config.recoveryManager,
        recoveryTimelock: config.recoveryTimelock,
        optimisticSeconds: config.optimisticSeconds,
        processGas: config.processGas ?? 850_000,
        reserveGas: config.reserveGas ?? 15_000,
      },
    ];
  }

  static toRustConfigs(deploys: CoreDeploy[]): RustConfig[] {
    let configs: RustConfig[] = [];
    for (let i = 0; i < deploys.length; i++) {
      const local = deploys[i];

      // copy array so original is not altered
      const remotes = deploys
        .slice()
        .filter((remote) => remote.chain.domain !== local.chain.domain);

      // build and add new config
      configs.push(CoreDeploy.buildConfig(local, remotes));
    }
    return configs;
  }

  static buildConfig(local: CoreDeploy, remotes: CoreDeploy[]): RustConfig {
    const home = {
      address: local.contracts.home!.proxy.address,
      domain: local.chain.domain.toString(),
      name: local.chain.name,
      rpcStyle: 'ethereum',
      connection: {
        type: 'http',
        url: '',
      },
    };

    const rustConfig: RustConfig = {
      environment: local.config.environment,
      signers: {
        [home.name]: { key: '', type: 'hexKey' },
      },
      replicas: {},
      home,
      tracing: {
        level: 'debug',
        fmt: 'json',
      },
      db: 'db_path',
    };

    for (var remote of remotes) {
      const replica = {
        address: remote.contracts.replicas[local.chain.domain].proxy.address,
        domain: remote.chain.domain.toString(),
        name: remote.chain.name,
        rpcStyle: 'ethereum',
        connection: {
          type: 'http',
          url: '',
        },
      };

      rustConfig.signers[replica.name] = { key: '', type: 'hexKey' };
      rustConfig.replicas[replica.name] = replica;
    }

    return rustConfig;
  }

  static freshFromConfig(chainConfig: ChainJson & CoreConfig): CoreDeploy {
    let [chain, config] = CoreDeploy.parseCoreConfig(chainConfig);
    return new CoreDeploy(chain, config);
  }

  static fromDirectory(
    directory: string,
    chain: Chain,
    config: CoreConfig,
    test: boolean = false,
  ): CoreDeploy {
    let deploy = new CoreDeploy(chain, config, test);
    const addresses: CoreDeployAddresses = JSON.parse(
      readFileSync(
        path.join(directory, `${chain.name}_contracts.json`),
      ) as any as string,
    );
    deploy.contracts = CoreContracts.fromAddresses(addresses, chain.provider);
    deploy.verificationInput = getVerificationInputFromDeploy(
      directory,
      chain.config.name,
    );
    return deploy;
  }
}

// The accessors is necessary as a network may have multiple core configs
export function makeCoreDeploys<V>(
  directory: string,
  data: V[],
  chainAccessor: (data: V) => Chain,
  coreConfigAccessor: (data: V) => CoreConfig,
): CoreDeploy[] {
  return data.map((d: V) =>
    CoreDeploy.fromDirectory(
      directory,
      chainAccessor(d),
      coreConfigAccessor(d),
    ),
  );
}