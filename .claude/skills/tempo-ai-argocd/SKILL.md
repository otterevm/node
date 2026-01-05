---
name: tempo-ai-argocd
description: Manages ArgoCD applications via Tailscale. Use when asked about deployments, syncing apps, rollbacks, or checking app status in dev/staging/prod environments.
---

# ArgoCD Operations

Manage ArgoCD applications across environments discovered via Tailscale.

## Prerequisites

- `tempo-argocd` CLI installed (run `./install.sh` from tempo-ai repo)
- Connected to Tailscale

## Commands

```bash
# Discover environments
tempo-argocd servers

# All commands: tempo-argocd <env> <command> [args]
tempo-argocd <env> list                  # List all apps
tempo-argocd <env> get <app>             # App details
tempo-argocd <env> sync <app>            # Sync app
tempo-argocd <env> diff <app>            # Preview changes
tempo-argocd <env> logs <app>            # Pod logs
tempo-argocd <env> history <app>         # Deployment history
tempo-argocd <env> rollback <app> [rev]  # Rollback
tempo-argocd <env> health                # Server health
```

## Workflow

1. Run `tempo-argocd servers` to see available environments
2. If user mentions "dev", "prod", "staging" - ask which specific environment (e.g., dev-euw, prd-nae)
3. Run command with environment prefix

## Authentication

If output shows "NOT AUTHENTICATED", tell user to run the login command shown:

```
argocd login <server-fqdn> --sso --grpc-web
```

Then retry.

## Examples

```bash
# Check out-of-sync apps in dev
tempo-argocd dev-euw list | grep OutOfSync

# Deploy workflow
tempo-argocd dev-euw diff my-app
tempo-argocd dev-euw sync my-app
tempo-argocd dev-euw get my-app

# Rollback in prod
tempo-argocd prd-nae history my-app
tempo-argocd prd-nae rollback my-app 5
```
