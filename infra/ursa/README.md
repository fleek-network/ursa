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
  - etsencrypt keys, and certbot web data will be exposed as volumes to be configure nginx, and to share data with the certbot service.
  - the location /.will-known/acme-challenege/ which certbot is going to use to negotiate with the letsencrypt servers to generate our certficate. 
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