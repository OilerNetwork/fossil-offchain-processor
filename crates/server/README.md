# Fossil API server

## Building the Docker image

In order to build the docker image, you'll have to build from workspace root and pointing to the `Dockerfile` in this crate directory.

If you are building on an OS that is not of x86 (e.g. arm), you might need to build for a x86 image since the `ndarray-linalg` library rely on some binaries that is only compatible with x86 architecture.

Your build command should look something along the lines of something like this:

```bash
docker build --platform linux/amd64 -t fossil-api -f crates/server/Dockerfile . 
```

## Running the server locally with docker compose

You can run the whole of the api server locally by using docker compose. This will spin up 3 components:

1. The database
2. The migration service which will terminate once migration is completed
3. The service itself

First, you'll need to set the environment variables in the `.env` file. Create a copy of `.env.example` (root) into the `crates/server` directory, name it `.env` and fill in the values.

Now you can run the components with:

```bash
docker compose -f ./crates/server/docker-compose.yml up
```

Note that you should run this command in the root of the workspace.

## Running the server via docker compose in deployment

In deployment, you can re-use most of what's in the local setup for `docker-compose.yml`, however you might want to remove the `db` service as you might prefer that to be hosted at some other location with better access to available resources.

You might also want to remove the `migration` service if you prefer migrations to be more manual.
