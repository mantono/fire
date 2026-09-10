# fire
A HTTP request client for your terminal

## Usage
##### Execute a request
`fire my_request.yml`

##### Execute a request for a specific environment
`fire my_request.yml -e environment`

## Request Files
A request file uses [YAML](https://quickref.me/yaml) (`.yml`) syntax and contains the following properties

| Property | Required | Example Value |
|:--------:|:--------:|:-------------:|
| method   | **Yes**  | `POST`        |
| url      | **Yes**  | `https://42x.io/some-endpoint` |
| headers  | No       | `content-type: application/json` |
| body     | No       | `{ "foo": "bar" }` |

```yaml
# This is a comment that can be used as a description for the request file
method: POST
url: https://42x.io/some-endpoint
headers:
  accept: application/json
  # Headers `host`, `content-length` (if there is a body) and `user-agent` are always sent
  # since a lot of requests will fail without them.
  content-type: application/json
  x-correlation-id: ce19f5b6-6333-4004-a191-67476fe241be

body: |
  {
    "foo": "bar",
    "nice_primes": [977, 3457, 3457, 6133, 7919]
  }
```

A more complex example with templating (using [Handlebars syntax](https://handlebarsjs.com/guide/#what-is-handlebars))

```yaml
method: GET
url: https://{{DOMAIN_NAME}}/some-endpoint?user={{USER}}
headers:
  accept: application/json
  authorization: Bearer {{TOKEN}}
```

See [examples](examples/) directory for more examples of how to structure request files, including static and command default values (see [Default Values](#default-values) below).

## Templating and Variable Substitution
Request files supports templating where variables can be substituted at execution time. This makes it very easy to have request
files that can be re-used for different environments or contexts. Variables can be read from the following sources (from least priority
to highest priority):
1. Environment variables - environment variables in your system are accessed by the application and will be used, but only as last resort
if they cannot be found anywhere else
2. Global environment files - any file named exactly `.env` or `.sec` which is present in the same directory or parent directories of the request file
3. Environment specific files - any file matching the specific environment (like `development.env` or `development.sec` if the environment is `development`) which is present in the same directory or parent directories of the request file, when an environment is specified (using flag `-e`)
4. Command line arguments - variables supplied to the application at the command line at execution time (using flag `-E`), these will override any of the previous sources for variables
5. Interactive mode - when the application is run in interactive mode (using flag `-i`) the user may provide values for template keys where a value is _missing_, i.e. it could not be found from any of the four sources above

The priority of resolved environment files are such as that the any environment file in the same folder as the request file has the highest priority when resolving variables (if the same variable is defined in multiple places). Found files in parent directories will be considered as well, but the futher up they are found the lower priority they will have. If the request files are stored in a Git repository, the application will never consider files outside the repository. If the request is not stored in a Git repository, only the immediate directory and no parents will be considered.

It is possible to specify multiple environments with the `--environment` (`-e`) flag, if needed. The order in which the different environments variables are applied are undefined, so it is recommended that you do not have the same variable defined in multiple environments if you are going to use them at the same time.

Note that there is technically not any difference between files ending with `.env` and `.sec`. Having two different file endings for environment configuration allows you to have the convention of putting sensitive variables in `.sec` files while having non-sensitive configuration in `.env` files. **It is highly recommended that you add a `.gitignore` filter to your repository which is `*.sec`, so you do not accidentally commit any secrets.**

Example of file which contains "environment variables"

```sh
API_URL=https://url-to-some.api.com/api
REGION=europe
USERNAME="quoted-username"
```

### Default Values
A template key may declare a default value that is used only when no value can be resolved for it from any of the sources above. Two syntaxes are supported:

- Static default: `{{NAME:value}}` - if `NAME` is not supplied, the literal `value` is used.
- Command (dynamic) default: `{{NAME|command}}` - if `NAME` is not supplied, `command` is executed through the system shell and its normalized output is used.

```yaml
method: POST
url: https://example.com
headers:
  content-type: {{CONTENT_TYPE:application/json}}
  x-correlation-id: {{CORRELATION_ID|uuidgen}}
```

**Source precedence is unchanged by defaults.** A default is only a fallback of last resort before interactive prompting: any value found via a CLI `-E` argument, an environment/secret file, or an inherited system environment variable is always used instead of a default, regardless of which of those sources it came from. A sourced value that is explicitly empty (`""`) still counts as resolved and suppresses the default entirely, including a command default, so the command is never invoked in that case.

**Defaults are resolved per occurrence, not per name.** Each `{{NAME...}}` occurrence in a request file is resolved independently, so the same `NAME` may appear multiple times with different (or no) fallback in each place:

```yaml
# All three occurrences of ID render as "value" if ID is supplied.
# Otherwise the first has no default, so it raises the legacy missing-value
# error (or prompts, in interactive mode), the second renders "a", and the
# third renders "b".
headers:
  x-a: {{ID}}
  x-b: {{ID:a}}
  x-c: {{ID:b}}
```

**Ordering with interactive mode.** Defaults are always resolved before interactive prompting (`-i`/`--interactive`). Any occurrence with a static or command default is therefore never prompted for, even in interactive mode; only a plain `{{NAME}}` occurrence with no default and no resolved value can trigger a prompt, and only as the last resort.

**Command execution details.** A command default requires the explicit opt-in flag `--allow-command-fallbacks`; without it, reaching a command default is a hard error and the command is never invoked. The command itself is run as a single argument to the platform shell: `sh -c "command"` on Unix and `cmd /C "command"` on Windows. Its standard output is captured as UTF-8; only a trailing `\r` and/or `\n` sequence is stripped, all other whitespace in the output is preserved as-is. A shell that cannot be launched, exits with a non-zero status, or produces output that is not valid UTF-8 each produce a distinct, non-secret error instead of silently falling back to any other value.

> **Security warning:** `--allow-command-fallbacks` causes `fire` to execute arbitrary shell commands found in the request file you are running, without confirmation. Only enable this flag for request files you trust, since a command default behaves the same as running that command yourself in a shell.

See [`examples/request_with_static_default.yml`](examples/request_with_static_default.yml) and [`examples/request_with_command_default.yml`](examples/request_with_command_default.yml) for minimal, non-executing examples of each syntax.

## Additional Documentation
See `fire --help` for more documentation on how to use the application.

## Building
1. [Install Rust](https://rustup.rs/) - if you do not have it installed
2. Clone this repository - `git clone git@github.com:mantono/fire.git`
3. Build application - `cargo build --release` inside the root of the repository
4. Install application - `cargo install --path .`
