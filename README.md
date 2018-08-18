# Outline

The scope of the project is to create a secure stopgap between a CI system and a secure
production environment. This software will should prevent a bad actor from stealing
secrets or gaining access to the production environment by gaining access to the
repository or CI environment.

## Background

It's really convenient to use central code hosting with CI runs to both validate and
provision the production environment. If either of the central code hosting or CI
environment is hacked the bad actor will have full access to your production environment.

## Solution

Do **not** trust the repository, **or** the CI system. Move the trust to your computer with GPG
signed commits. Then in the other end, setup Outline as a service that does the actual
work.

Outline is a simple daemon that **only** runs if specified commit is trusted.

Workflow:

1. A trusted person signs his or her git commit
2. The code is committed to a git repository
3. A CI run is started, it sends what repository, commit and command to execute on Outline
4. Outline validates the GPG signed commit, and only if it's trusted it will execute the specified command.

You can't specify artitary commands, there is a whitelist.

## Protocol

Outline listend to `$HOST:$PORT` with will default to localhost:8080 by default.
It expect you to open a TCP connection and send the following commands in order:

* URL of the repository to clone
* Commit hash to checkout
* The command to execute

An example of this is:

```
git.example.com/repo.git
423a975db97a42823e0d6c730d675390a1c785b4
make
```

From this point forward outline will ignore all input, stdout and stderr will be streamed back to you. Outline is simple to call from your existing CI system with for example the `nc` command.

```sh
echo -e "git.example.com/repo.git\n423a9...785b4\nmake" | nc localhost 8080
```

## The service

Run the service in your prefered way, it's configured from the environment.

* `PORT` Used to change the port, default is `8080`.
* `HOST` Used to change the host/ip it's listens at. It's defaults to `localhost` so you probably like to change this to `0.0.0.0`.
* `COMMANDS` A comma separated list of allowed commands, it's defaults to `make,make check`
* `INSECURE` Set this to `1` to disable both GPG and the command whitelist. Only use this for debug. It defaults to `0`.
* `PREFIX` Prefix all commands, this can be useful if you like to run commands inside containers. Defaults to empty string.

Configure your local git installation to trust your signed commits.