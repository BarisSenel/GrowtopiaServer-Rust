(Yes below is AI generated, heres quick look, just copy repo there is login system on port 443 host it on somewhere i used cloudflared with my own domain, create discord bot put details on .env file edit gameserver_adress as your public or local ip and run it, if you need help ask me at discord you can find in my profile)

# GrowtopiaServer-Rust Setup Guide

> **Note**: This project was made with **VibeCode** for testing and development purposes.
> Heres the repos i mostly looked at to make this project:
> - https://github.com/StileDevs/GrowServer
> - https://github.com/gurotopia/Gurotopia
> - https://github.com/StileDevs/growtopia.js/ (for enet)



This document provides a comprehensive guide to building, configuring, and running the Growtopia Private Server. Follow these steps sequentially to set up your server environment.

## 1. Installation & Build

First, you need to obtain the server executable.

### Option A: Build from Source (Recommended)
You must have Rust installed.
1.  **Clone the Repository**:
    ```bash
    git clone https://github.com/BarisSenel/GrowtopiaServer-Rust.git
    cd GrowtopiaServer-Rust
    ```
2.  **Build Release**:
    ```bash
    cargo build --release
    ```
    The executable will be located at `target/release/GrowServer.exe` (Windows) or `target/release/GrowServer` (Linux).

---

## 2. Server Preparation

Before running the server, ensure your working directory contains the necessary files.

### A. Required Files
1.  **items.dat**: Download a valid `items.dat` file (version 19 or higher) and place it in the same folder as `GrowServer.exe`.
2.  **SSL Certificates**: The server requires `key.pem` and `cert.pem`. We recommend using **mkcert** to generate trusted certificates for development.
    - **Install mkcert**:
      - **Windows**: `choco install mkcert` (requires Chocolatey)
      - **Linux/macOS**: `brew install mkcert` (requires Homebrew)
    - **Initialize Root CA**:
      ```bash
      mkcert -install
      ```
    - **Generate Certificates**:
      Run the following command to generate certificates for your custom domain and the required Growtopia wildcard domains:
      ```bash
      mkcert -key-file key.pem -cert-file cert.pem "yourdomain.com" "*.yourdomain.com" "*.growtopia1.com" "*.growtopia2.com" localhost 127.0.0.1
      ```
    - **Note**: If your server setup requires PKCS8 format (sometimes needed for Rust `native-tls` vs `rustls`), you can convert the key using OpenSSL:
      ```bash
      openssl pkcs8 -topk8 -inform PEM -outform PEM -in key.pem -out key_pkcs8.pem -nocrypt
      ```

---

## 3. Configuration & Login Setup

The server requires a public HTTPS domain to handle Discord OAuth logins securely. We use **Cloudflared** for this purpose.

### Part A: Domain Setup (Cloudflared)
1.  **Install Cloudflared**: Download and install the Cloudflared tool.
2.  **Authenticate**: Run `cloudflared tunnel login`.
3.  **Create a Tunnel**: Run `cloudflared tunnel create growserver`. Note the Tunnel UUID.
4.  **Configure Tunnel**:
    Create a `config.yml` in your `.cloudflared` directory:
    ```yaml
    tunnel: <YOUR_TUNNEL_UUID>
    credentials-file: C:\Users\YourUser\.cloudflared\<YOUR_TUNNEL_UUID>.json

    ingress:
      - hostname: yourdomain.com
        service: https://localhost:443
        originRequest:
          noTLSVerify: true
      - hostname: www.yourdomain.com
        service: https://localhost:443
        originRequest:
          noTLSVerify: true
      - service: http_status:404
    ```
5.  **Route DNS**: Run `cloudflared tunnel route dns growserver yourdomain.com`.
6.  **Start Tunnel**: Run `cloudflared tunnel run growserver`.

### Part B: Discord Application (OAuth)
1.  Go to the **Discord Developer Portal**.
2.  Create a **New Application**.
3.  Go to **OAuth2**.
4.  Add `https://yourdomain.com/discord/callback` to the **Redirects** list.
5.  Copy your **Client ID** and **Client Secret**.

### Part C: Environment Variables (.env)
Create a `.env` file in the server folder:

```ini
# Server Configuration
# This must be your PUBLIC IP address if hosting on a VPS/Dedicated Server.
# If running locally for testing, use 127.0.0.1
gameserver_adress=YOUR_PUBLIC_IP
gameserver_port=17091

# The domain you configured with Cloudflared
webserver_adress=yourdomain.com

GAMESERVER_TOKEN=choose_a_random_token

# Discord Login Configuration
DISCORD_CLIENT_ID=<Paste Client ID Here>
DISCORD_CLIENT_SECRET=<Paste Client Secret Here>
DISCORD_REDIRECT_URI=https://yourdomain.com/discord/callback
host_login_url=yourdomain.com
```

---

## 4. Running the Server

1.  Open a terminal in your server folder.
2.  Run the server:
    ```bash
    ./GrowServer.exe
    ```
3.  The server should start and listen on port `17091` (UDP) and `443` (HTTP/HTTPS via tunnel).

---

## 5. Connecting to the Server (Client Setup)

Players must redirect Growtopia's traffic to your server IP.

### Windows (Hosts File)
1.  Open **Notepad** as Administrator.
2.  Open `C:\Windows\System32\drivers\etc\hosts`.
3.  Add the following lines at the bottom, replacing `YOUR_VPS_IP` with the server's public IP address:
    ```
    YOUR_VPS_IP www.growtopia1.com
    YOUR_VPS_IP www.growtopia2.com
    ```
4.  Save the file.
5.  Open the Growtopia client and connect.
