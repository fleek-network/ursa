.PHONY: init plan apply destroy fmt init-tf plan-tf apply-tf destroy-tf nodes-tf ping-tf exec-tf

TF_IPS=`jq -rc '.outputs.testnet_nodes_ip_address.value[]' tf/terraform.tfstate`

init:
	@cd tf-next && terraform init
	@cp tf-next/production.tfvars.example tf-next/production.tfvars

plan:
	@cd tf-next && terraform plan -var-file="production.tfvars"

apply:
	@cd tf-next && terraform apply -auto-approve -var-file="production.tfvars"

destroy:
	@cd tf-next && terraform destroy -var-file="production.tfvars"

fmt:
	@cd tf-next && terraform fmt
	@cd tf-next/k8s && terraform fmt
	@cd tf && terraform fmt

init-tf:
	@cd tf && terraform init

plan-tf:
	@cd tf && terraform plan

apply-tf:
	@cd tf && terraform apply

destroy-tf:
	@cd tf && terraform destroy

nodes-tf:
	@echo $(TF_IPS)

ping-tf:
	@for i in $(TF_IPS); do \
    printf "$$i ... "; \
  	if [[ "`curl -sm 3 http://$$i/ping`" ]]; then \
      printf "ok\n"; \
    else \
      printf "down\n"; \
    fi; \
  done

exec-tf:
	@for i in $(TF_IPS); do \
		echo "$$i:"; \
		ssh root@$$i -o StrictHostKeyChecking=no "$s"; \
		echo; \
	done
