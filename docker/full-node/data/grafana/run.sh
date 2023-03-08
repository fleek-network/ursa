#!/bin/bash

# fetch node-exporter-full dashboard
wget https://raw.githubusercontent.com/rfmoz/grafana-dashboards/master/prometheus/node-exporter-full.json -O /etc/grafana/dashboards/node-exporter-full.json

# startup grafana
/run.sh
