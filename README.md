# fire
## Usage
##### Execute a request
`fire my_request.req`

See examples directory for how to structure request files.

##### Execute a request with custom environment variables
`fire my_request.req -e environment.env`

Enviroment variables files has the following format

```sh
API_URL=https://url-to-some.api.com/api
TOKEN=some-secret-token
USERNAME="quoted-username"
```
