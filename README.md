# fire
A HTTP request client for your terminal

## Usage
##### Execute a request
`fire my_request.req`

##### Execute a request for a specific environment
`fire my_request.req -e environment`

## Request Files
A request (`.req`) file contains the following things
- Method (`GET`, `POST`, `PUT`, etc)
- URL
- Headers (optional)
- Body (optional)
- Decription or comments (optional)

The body is separated from the headers with one blank line.

```yaml
# This is comment that can be used as a description for the request file
# Comments can be placed anywhere in the file except for inside the body
# The first line of the file (if it not a comment) must contain the method and the URL
POST https://42x.io/some-endpoint
accept: application/json
# Headers `host`, `content-length` (if there is a body) and `user-agent` are always sent
# since a lot of requests will fail without them.
content-type: application/json
x-correlation-id: ce19f5b6-6333-4004-a191-67476fe241be
# Note that the next blank line is significant for being able to parse the body

{
  "foo": "bar",
  "nice_primes": [977, 3457, 3457, 6133, 7919]
}
```

A more complex example with templating (using [Handlebars syntax](https://handlebarsjs.com/guide/#what-is-handlebars))

```yaml
GET https://{{DOMAIN_NAME}}/some-endpoint?user={{USER}}
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

The priority of resolved environment files are such as that the any environment file in the same folder as the request file has the highest priority when resolving variables (if the same variable is defined in multiple places). Found files in parent directories will be considered as well, but the futher up they are found the lower priority they will have. If the request files are stored in a Git repository, the application will never consider files outside the repository. If the request is not stored in a Git repository, only the immediate directory and no parents will be considered.

It is possible to specify multiple environments with the `--environment` (`-e`) flag, if needed. The order in which the different environments variables are applied are undefined, so it is recommended that you do not have the same variable defined in multiple environments if you are going to use them at the same time.

Note that there is technically not any difference between files ending with `.env` and `.sec`. Having two different file endings for environment configuration allows you to have the convention of putting sensitive variables in `.sec` files while having non-sensitive configuration in `.env` files. **It is highly recommended that you add a `.gitignore` filter to your repository which is `*.sec`, so you do not accidentally commit any secrets.**

Example of file which contains "environment variables"

```sh
API_URL=https://url-to-some.api.com/api
REGION=europe
USERNAME="quoted-username"
```

## Additional Documentation
See `fire --help` for more documentation on how to use the application.

## Building
1. [Install Rust](https://rustup.rs/) - if you do not have it installed
2. Clone this repository - `git clone git@github.com:mantono/fire.git`
3. Build application - `cargo build --release` inside the root of the repository
4. Install application - `cargo install --path .`
