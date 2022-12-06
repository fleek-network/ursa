## Deployment notes

### Requirements
- `Ubuntu 22.04 or later.`
- `8GB memory is recommended but 4GB should be enough for alpha.`	
- `A domain name.`

### Digital Ocean

1. Create Droplet and set up the basic for a user to run.
    - Setup [Initial server setup](https://www.digitalocean.com/community/tutorials/initial-server-setup-with-ubuntu-22-04)
    - install [docker](https://www.digitalocean.com/community/tutorials/how-to-install-and-use-docker-on-ubuntu-22-04)
        -  Setup [buildkit](https://docs.docker.com/develop/develop-images/build_enhancements/)

2. Point your Domain nameservers to Digital ocean.

3. Clone the repo and build with docker-compose
  ```sh
  make compose-up 

  # or

  docker-compose -f infra/ursa/docker-compose.yml up

  # if build kit isn't installed
  COMPOSE_DOCKER_CLI_BUILD=1 DOCKER_BUILDKIT=1 docker-compose up -d
  ```

4. Setup TLS.

- Find and Replace all domain instances in `data/nginx/app.conf` and `init-letsencrypt.sh` with your domain.

- Generate Certificates for your domain and restart nginx

  ```sh
  bash infra/ursa/init-letsencrypt.sh
  ```

- If you have problems during the setup, you can try installing `certbot` locally and then run `sudo certbot certonly --standalone -d domain.com -d www.domain.com` and then move cert and privkey to the correct place.

### Test your endpoint

You can test locally by replacing `<HOSTNAME>` by `localhost` and `<PORT>` as `4069`. Replace The `<VALID-CID-HERE>` with a valid [CID](https://docs.ipfs.tech/concepts/content-addressing/) (content identifier). The flag `-o` refers to `output` to a custom filename.

```
curl -X GET http://<HOSTNAME>:<PORT>/<VALID-CID-HERE> -o <FILENAME>
```

Here is an example

```
curl -X GET http://localhost:4069/bafybeifyjj2bjhtxmp235vlfeeiy7sz6rzyx3lervfk3ap2nyn4rggqgei  -o my_file.car
```