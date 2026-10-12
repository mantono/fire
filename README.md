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

See [examples](examples/) directory for more examples of how to structure request files.

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

### Values from shell commands

In `.env` and `.sec` files, a value can use `$(command)` to capture a shell command's standard output:

```sh
REGION=europe
id=$(uuidgen)
label=$(printf '%s' 'request-label')
```

Fire resolves assignments once, from top to bottom, when it loads each file.
Commands cannot reference values assigned earlier in the same file.
Every `{{id}}` in a request uses the same resolved value.
Commands run even when a variable is unused or a higher-priority source overrides it.
The existing variable-source priority does not change.

After the existing removal of whitespace and surrounding quotes, Fire treats values starting with `$(` and ending with `)` as command assignments.
Use a single `$(command)` as the whole value.
Combined substitutions, such as `$(printf a)-$(printf b)`, are unsupported and can fail loading.
Values that do not match these boundaries, such as `prefix-$(uuidgen)`, remain literal text.
Static assignments, command-line values, and system environment values keep their existing behavior.
Request templates do not execute commands.

Commands use `sh -c` and run in Fire's current working directory.
They inherit only Fire's system environment. File assignments do not override inherited values.
Assignments do not change Fire's system environment or carry over to commands in the same file or other files.
Fire removes trailing newline characters from the output but preserves other whitespace and internal newlines.

If a command cannot start, exits unsuccessfully, or returns output that is not valid UTF-8 text, Fire stops before sending the request.
The error names the variable without including the command or its output.

Only load environment and secret files that you trust.
Their commands run automatically with your user permissions and can read secrets, modify files, or access the network.

## Additional Documentation
See `fire --help` for more documentation on how to use the application.

## Building
1. [Install Rust](https://rustup.rs/) - if you do not have it installed
2. Clone this repository - `git clone git@github.com:mantono/fire.git`
3. Build application - `cargo build --release` inside the root of the repository
4. Install application - `cargo install --path .`
