
// this file is generated — do not edit it


/// <reference types="@sveltejs/kit" />

/**
 * This module provides access to environment variables that are injected _statically_ into your bundle at build time and are limited to _private_ access.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Static environment variables are [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env` at build time and then statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * **_Private_ access:**
 * 
 * - This module cannot be imported into client-side code
 * - This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured)
 * 
 * For example, given the following build time environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://site.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { ENVIRONMENT, PUBLIC_BASE_URL } from '$env/static/private';
 * 
 * console.log(ENVIRONMENT); // => "production"
 * console.log(PUBLIC_BASE_URL); // => throws error during build
 * ```
 * 
 * The above values will be the same _even if_ different values for `ENVIRONMENT` or `PUBLIC_BASE_URL` are set at runtime, as they are statically replaced in your code with their build time values.
 */
declare module '$env/static/private' {
	export const SVELTEKIT_FORK: string;
	export const NODE_ENV: string;
	export const CODEX_INTERNAL_ORIGINATOR_OVERRIDE: string;
	export const COLORTERM: string;
	export const npm_config_prefix: string;
	export const npm_node_execpath: string;
	export const NODE_REPL_TRUSTED_CODE_PATHS: string;
	export const RUBYMINE_VM_OPTIONS: string;
	export const RIDER_VM_OPTIONS: string;
	export const _P9K_SSH_TTY: string;
	export const NODE_REPL_TRUSTED_BROWSER_CLIENT_SHA256S: string;
	export const npm_config_cache: string;
	export const MY_RELAY_API_KEY: string;
	export const SHLVL: string;
	export const STUDIO_VM_OPTIONS: string;
	export const JETBRAINSCLIENT_VM_OPTIONS: string;
	export const AQUA_VM_OPTIONS: string;
	export const XPC_SERVICE_NAME: string;
	export const npm_package_version: string;
	export const HOMEBREW_CELLAR: string;
	export const DATASPELL_VM_OPTIONS: string;
	export const ZSH_TMUX_AUTOSTARTED: string;
	export const DATAGRIP_VM_OPTIONS: string;
	export const WEBSTORM_VM_OPTIONS: string;
	export const npm_config_npm_version: string;
	export const CLION_VM_OPTIONS: string;
	export const _CONDA_EXE: string;
	export const LANG: string;
	export const EDITOR: string;
	export const npm_lifecycle_event: string;
	export const CODEX_THREAD_ID: string;
	export const LESS: string;
	export const GH_PAGER: string;
	export const DISABLE_AUTO_UPDATE: string;
	export const JAVA_HOME: string;
	export const npm_config_globalconfig: string;
	export const XPC_FLAGS: string;
	export const npm_command: string;
	export const GATEWAY_VM_OPTIONS: string;
	export const MallocNanoZone: string;
	export const LSCOLORS: string;
	export const CODEX_CI: string;
	export const npm_lifecycle_script: string;
	export const npm_package_json: string;
	export const TMPDIR: string;
	export const HOMEBREW_API_DOMAIN: string;
	export const ZSH_TMUX_AUTOSTART: string;
	export const LC_CTYPE: string;
	export const _CONDA_ROOT: string;
	export const npm_config_userconfig: string;
	export const PATH: string;
	export const FPATH: string;
	export const CONDA_PYTHON_EXE: string;
	export const npm_config_python: string;
	export const PYENV_SHELL: string;
	export const RUSTROVER_VM_OPTIONS: string;
	export const PAGER: string;
	export const npm_execpath: string;
	export const __CF_USER_TEXT_ENCODING: string;
	export const PYCHARM_VM_OPTIONS: string;
	export const P9K_SSH: string;
	export const CODEX_PERMISSION_PROFILE: string;
	export const JETBRAINS_CLIENT_VM_OPTIONS: string;
	export const LS_COLORS: string;
	export const PHPSTORM_VM_OPTIONS: string;
	export const SSH_AUTH_SOCK: string;
	export const CONDA_EXE: string;
	export const LC_ALL: string;
	export const COMMAND_MODE: string;
	export const NVM_DIR: string;
	export const USER: string;
	export const TERM: string;
	export const npm_config_global_prefix: string;
	export const BROWSER_USE_AVAILABLE_BACKENDS: string;
	export const npm_package_name: string;
	export const npm_config_noproxy: string;
	export const DEVECOSTUDIO_VM_OPTIONS: string;
	export const SHELL: string;
	export const COLOR: string;
	export const npm_config_user_agent: string;
	export const NVM_BIN: string;
	export const BUN_INSTALL: string;
	export const INFOPATH: string;
	export const HOMEBREW_BOTTLE_DOMAIN: string;
	export const IDEA_VM_OPTIONS: string;
	export const NO_COLOR: string;
	export const INIT_CWD: string;
	export const RUST_LOG: string;
	export const CONDA_SHLVL: string;
	export const NODE: string;
	export const _CE_M: string;
	export const npm_config_node_gyp: string;
	export const PWD: string;
	export const NVM_CD_FLAGS: string;
	export const LOGNAME: string;
	export const CODEX_SHELL: string;
	export const npm_config_local_prefix: string;
	export const WEBIDE_VM_OPTIONS: string;
	export const MAVEN: string;
	export const _CE_CONDA: string;
	export const LOG_FORMAT: string;
	export const ZSH: string;
	export const HOMEBREW_PIP_INDEX_URL: string;
	export const npm_config_init_module: string;
	export const HOME: string;
	export const NVM_INC: string;
	export const _: string;
	export const npm_config_registry: string;
	export const GOLAND_VM_OPTIONS: string;
	export const HOMEBREW_PREFIX: string;
	export const GIT_PAGER: string;
	export const HOMEBREW_REPOSITORY: string;
	export const PYENV_ROOT: string;
}

/**
 * This module provides access to environment variables that are injected _statically_ into your bundle at build time and are _publicly_ accessible.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Static environment variables are [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env` at build time and then statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * **_Public_ access:**
 * 
 * - This module _can_ be imported into client-side code
 * - **Only** variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`) are included
 * 
 * For example, given the following build time environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://site.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { ENVIRONMENT, PUBLIC_BASE_URL } from '$env/static/public';
 * 
 * console.log(ENVIRONMENT); // => throws error during build
 * console.log(PUBLIC_BASE_URL); // => "http://site.com"
 * ```
 * 
 * The above values will be the same _even if_ different values for `ENVIRONMENT` or `PUBLIC_BASE_URL` are set at runtime, as they are statically replaced in your code with their build time values.
 */
declare module '$env/static/public' {
	
}

/**
 * This module provides access to environment variables set _dynamically_ at runtime and that are limited to _private_ access.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Dynamic environment variables are defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://svelte.dev/docs/kit/cli)), this is equivalent to `process.env`.
 * 
 * **_Private_ access:**
 * 
 * - This module cannot be imported into client-side code
 * - This module includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured)
 * 
 * > [!NOTE] In `dev`, `$env/dynamic` includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 * 
 * > [!NOTE] To get correct types, environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * >
 * > ```env
 * > MY_FEATURE_FLAG=
 * > ```
 * >
 * > You can override `.env` values from the command line like so:
 * >
 * > ```sh
 * > MY_FEATURE_FLAG="enabled" npm run dev
 * > ```
 * 
 * For example, given the following runtime environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://site.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { env } from '$env/dynamic/private';
 * 
 * console.log(env.ENVIRONMENT); // => "production"
 * console.log(env.PUBLIC_BASE_URL); // => undefined
 * ```
 */
declare module '$env/dynamic/private' {
	export const env: {
		SVELTEKIT_FORK: string;
		NODE_ENV: string;
		CODEX_INTERNAL_ORIGINATOR_OVERRIDE: string;
		COLORTERM: string;
		npm_config_prefix: string;
		npm_node_execpath: string;
		NODE_REPL_TRUSTED_CODE_PATHS: string;
		RUBYMINE_VM_OPTIONS: string;
		RIDER_VM_OPTIONS: string;
		_P9K_SSH_TTY: string;
		NODE_REPL_TRUSTED_BROWSER_CLIENT_SHA256S: string;
		npm_config_cache: string;
		MY_RELAY_API_KEY: string;
		SHLVL: string;
		STUDIO_VM_OPTIONS: string;
		JETBRAINSCLIENT_VM_OPTIONS: string;
		AQUA_VM_OPTIONS: string;
		XPC_SERVICE_NAME: string;
		npm_package_version: string;
		HOMEBREW_CELLAR: string;
		DATASPELL_VM_OPTIONS: string;
		ZSH_TMUX_AUTOSTARTED: string;
		DATAGRIP_VM_OPTIONS: string;
		WEBSTORM_VM_OPTIONS: string;
		npm_config_npm_version: string;
		CLION_VM_OPTIONS: string;
		_CONDA_EXE: string;
		LANG: string;
		EDITOR: string;
		npm_lifecycle_event: string;
		CODEX_THREAD_ID: string;
		LESS: string;
		GH_PAGER: string;
		DISABLE_AUTO_UPDATE: string;
		JAVA_HOME: string;
		npm_config_globalconfig: string;
		XPC_FLAGS: string;
		npm_command: string;
		GATEWAY_VM_OPTIONS: string;
		MallocNanoZone: string;
		LSCOLORS: string;
		CODEX_CI: string;
		npm_lifecycle_script: string;
		npm_package_json: string;
		TMPDIR: string;
		HOMEBREW_API_DOMAIN: string;
		ZSH_TMUX_AUTOSTART: string;
		LC_CTYPE: string;
		_CONDA_ROOT: string;
		npm_config_userconfig: string;
		PATH: string;
		FPATH: string;
		CONDA_PYTHON_EXE: string;
		npm_config_python: string;
		PYENV_SHELL: string;
		RUSTROVER_VM_OPTIONS: string;
		PAGER: string;
		npm_execpath: string;
		__CF_USER_TEXT_ENCODING: string;
		PYCHARM_VM_OPTIONS: string;
		P9K_SSH: string;
		CODEX_PERMISSION_PROFILE: string;
		JETBRAINS_CLIENT_VM_OPTIONS: string;
		LS_COLORS: string;
		PHPSTORM_VM_OPTIONS: string;
		SSH_AUTH_SOCK: string;
		CONDA_EXE: string;
		LC_ALL: string;
		COMMAND_MODE: string;
		NVM_DIR: string;
		USER: string;
		TERM: string;
		npm_config_global_prefix: string;
		BROWSER_USE_AVAILABLE_BACKENDS: string;
		npm_package_name: string;
		npm_config_noproxy: string;
		DEVECOSTUDIO_VM_OPTIONS: string;
		SHELL: string;
		COLOR: string;
		npm_config_user_agent: string;
		NVM_BIN: string;
		BUN_INSTALL: string;
		INFOPATH: string;
		HOMEBREW_BOTTLE_DOMAIN: string;
		IDEA_VM_OPTIONS: string;
		NO_COLOR: string;
		INIT_CWD: string;
		RUST_LOG: string;
		CONDA_SHLVL: string;
		NODE: string;
		_CE_M: string;
		npm_config_node_gyp: string;
		PWD: string;
		NVM_CD_FLAGS: string;
		LOGNAME: string;
		CODEX_SHELL: string;
		npm_config_local_prefix: string;
		WEBIDE_VM_OPTIONS: string;
		MAVEN: string;
		_CE_CONDA: string;
		LOG_FORMAT: string;
		ZSH: string;
		HOMEBREW_PIP_INDEX_URL: string;
		npm_config_init_module: string;
		HOME: string;
		NVM_INC: string;
		_: string;
		npm_config_registry: string;
		GOLAND_VM_OPTIONS: string;
		HOMEBREW_PREFIX: string;
		GIT_PAGER: string;
		HOMEBREW_REPOSITORY: string;
		PYENV_ROOT: string;
		[key: `PUBLIC_${string}`]: undefined;
		[key: `${string}`]: string | undefined;
	}
}

/**
 * This module provides access to environment variables set _dynamically_ at runtime and that are _publicly_ accessible.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Dynamic environment variables are defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://svelte.dev/docs/kit/cli)), this is equivalent to `process.env`.
 * 
 * **_Public_ access:**
 * 
 * - This module _can_ be imported into client-side code
 * - **Only** variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`) are included
 * 
 * > [!NOTE] In `dev`, `$env/dynamic` includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 * 
 * > [!NOTE] To get correct types, environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * >
 * > ```env
 * > MY_FEATURE_FLAG=
 * > ```
 * >
 * > You can override `.env` values from the command line like so:
 * >
 * > ```sh
 * > MY_FEATURE_FLAG="enabled" npm run dev
 * > ```
 * 
 * For example, given the following runtime environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://example.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { env } from '$env/dynamic/public';
 * console.log(env.ENVIRONMENT); // => undefined, not public
 * console.log(env.PUBLIC_BASE_URL); // => "http://example.com"
 * ```
 * 
 * ```
 * 
 * ```
 */
declare module '$env/dynamic/public' {
	export const env: {
		[key: `PUBLIC_${string}`]: string | undefined;
	}
}
