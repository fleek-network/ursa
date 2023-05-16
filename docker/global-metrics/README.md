# Global Metrics

The global metrics provides a dashboard showing the overview of the network from the view of [Kademlia Exporter](https://github.com/mxinden/kademlia-exporter)

## Requirements

- Docker (with `compose`)
- [MaxMindDB](https://www.maxmind.com/en/home) for geo-lookups 

## Usage

### Clone the project

```sh
git clone https://github.com/fleek-network/ursa

cd ursa/docker/global-metrics
```

### Setup MaxMindDB

```sh
sudo apt-get install geoipupdate

sudo vim /etc/GeoIP.conf # paste credentials here
```

### Create the TLS Certificates

```sh
docker compose -f docker/global-metrics/docker-compose.yml \
  run \
  -p 80:80 \
  --rm --entrypoint "\
  certbot certonly \
    --standalone \
    --preferred-challenges http \
    --email <YOUR-EMAIL-ADDRESS> \
    --domain <YOUR-DOMAIN-NAME> \
    --rsa-key-size 4096 \
    --agree-tos -n" certbot
```

### Copy recommended TLS parameters (if missing)

```sh
curl -s curl -s https://raw.githubusercontent.com/certbot/certbot/master/certbot-nginx/certbot_nginx/_internal/tls_configs/options-ssl-nginx.conf > docker/global-metrics/certbot/conf/options-ssl-nginx.conf
```

```sh
curl -s https://raw.githubusercontent.com/certbot/certbot/master/certbot/certbot/ssl-dhparams.pem > docker/global-metrics/certbot/conf/ssl-dhparams.pem
```

### Nginx custom domain

Find and replace the `dashboard.ursa.earth` domain to your custom domain in `docker/global-metrics/data/nginx/app.conf`.

### Run the composition

```sh
docker compose up -d
```
