# Fly Deploy

- flyctl launch --image ipfs/go-ipfs:latest
- fly volumes create ipfs_data --region fra --size 3
- fly status
- fly ssh console -a ursa-ipfs
- fly regions set lax -a ursa-ipfs
- fly regions backup lax -a ursa-ipfs
- flyctl ips list

# Reverse Proxy Setup
- Host
  - It is assumed that DNS records point to the Docker host
  - add an A record with the actual domain

- **IPFS**: with local storage at `/data/ipfs` which is mounted locally for presistence.
  - Expose ports:
    - `4001` - ipfs swarm [public]
    - `8080` - ipfs gatewat [local]
    - `5001` - ipfs api [local]

- **Nginx**: should redirect 80 -> 443, but will listen on both. Then proxy the requests to our ipfs node.
  - letsencrypt keys, and certbot web data will be exposed as volumes to be configure nginx, and to share data with the certbot service.
  - the location `/.will-known/acme-challenege/` which certbot is going to use to negotiate with the letsencrypt servers to generate our certficate. 
  - Create nginx configration under `./data/nginx/app.conf`:
  - Listen on:
    - `80` - will redirect to HTTPS port 443
    - `443` - point to the certificate folder and nginx that we get from cerbot, then we proxy the traffic.

- **Cerbot**: will be used to generate and update our certificates
  - This shares two volumes with the nginx server to store the certificates themselves, as well as managing the web handshake 
  - download cerbot configration files under `./data/certbot` from `https://github.com/certbot/certbot`.
    - `options-ssl-nginx.conf`
    - `ssl-dhparams.pem`
  - Setup the initial certificates
    - nginx needs a certificate to startup SSL.
      - therefore create a fake cert at first which will later be replaced with certbot.
      - startup the nginx container, make sure its running, then remove the temp certs.
      - then start the certbot container

- **DNS**: 
  - Create an `A` record for the domain name pointing at the server running the node. Could also create `CNAME` records for `www` subdomains.

- Resources:
  - https://eff-certbot.readthedocs.io/en/stable/install.html#running-with-docker
  - https://www.digitalocean.com/community/tutorials/how-to-secure-nginx-with-let-s-encrypt-on-ubuntu-18-04
  - https://github.com/wmnnd/nginx-certbot/blob/master/init-letsencrypt.sh
  - https://github.com/wmnnd/nginx-certbot

  
  ## Deployment notes
  
  To deploy a node in DigitalOcean you can take the following notes.

* Basic requirements:
	- Ubuntu 22.04 or later. By the moment, 20.04 is getting errors during the build.
	- 4GB memory is recommended but 2GB should be enough.	- Real domain ownership.


1. Create Droplet and setup the basic for a user to run.
	- Setup user: https://www.digitalocean.com/community/tutorials/initial-server-setup-with-ubuntu-22-04
	- install docker: https://www.digitalocean.com/community/tutorials/how-to-install-and-use-docker-on-ubuntu-22-04
		-  Setup buildkit: https://docs.docker.com/develop/develop-images/build_enhancements/
2. Get a static ip address for your droplet with the available option at the panel.
3. Configure DNS to use your droplet and DigitalOcean name servers.
4. Clone the repo and build the image with `make docker-build`. You can use the `dev` command on the makefile but you also need to change the `infra/ursa/docker-compose.yml` to use `Dockerfile.dev`.
5. Setup TLS.
	- You need to change all the `ursanetwork.local` placeholders for your real domain and run the script in `infra/ursa/init-letsencrypt.sh`.
		- If you have problems during the setup, you can try installing `certbot` locally and then run `sudo certbot certonly --standalone -d domain.com -d www.domain.com` and then move cert and privkey to the correct place.

6. Once you have the DNS configured, TLS certs ready, and the image built, you can run the node by doing `make compose-up` or `docker-compose -f infra/ursa/docker-compose.yml up`


Now you are able to request your node.

```
curl -X POST https://domain.com/rpc/v0 \
 -H "Content-Type: application/json" \
 -d '{"jsonrpc": "2.0", "method": "ursa_get_cid", "params": {"cid": "some-valid-cid"}, "id": 1}'
```
  
  