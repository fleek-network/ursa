# Global Metrics

The global metrics provides a dashboard showing the overview of the network from the view of [Kademlia Exporter](https://github.com/mxinden/kademlia-exporter)

## Requirements

- Docker (with `compose`)
- [MaxMindDB](https://www.maxmind.com/en/home) for geo-lookups 

## Usage

Clone the project

```sh
git clone https://github.com/fleek-network/ursa

cd ursa/docker/global-metrics
```

Setup MaxMindDB

```sh
sudo apt-get install geoipupdate

sudo vim /etc/GeoIP.conf # paste credentials here
```

Run the composition

```sh
docker compose up -d
```
