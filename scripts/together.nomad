variable "domain" {
  type        = string
  default     = "kafi.work"
  description = "hostname" 
}

job "together" {
  datacenters = ["dc1"]
  type        = "service"

  group "together" {
    count = 1
    network {
      # port "http" {}
    }
    service {
      name = "together"
      #port = "http"

      tags = [
        "traefik.enable=true",
        "traefik.http.routers.bindle.rule=Host(`together.${var.domain}`)",
        "traefik.http.routers.bindle.tls.certresolver=mresolver",
        "traefik.http.routers.bindle.entrypoints=https",
        "traefik.http.routers.bindle.tls=true",
        "traefik.http.services.bindle.loadbalancer.server.port=8080"
      ]

      #check {
       # name     = "alive"
       # type     = "tcp"
       # interval = "10s"
       # timeout  = "2s"
      #}
    }

    task "together" {
      driver = "docker"
      env {
        RUST_LOG = "error"
      }
      config {
        image= "thankrain/together:1.0"
        network_mode = "bridge"
      }
    }
  }
}