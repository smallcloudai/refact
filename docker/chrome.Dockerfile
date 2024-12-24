FROM ubuntu:latest

# Install dependencies
RUN apt-get update && apt-get install -y \
    wget \
    gnupg \
    apt-transport-https \
    curl \
    net-tools \
    socat \
    && rm -rf /var/lib/apt/lists/*

# Add Google Chrome repository and install Google Chrome
RUN wget -q -O - https://dl.google.com/linux/linux_signing_key.pub | apt-key add - && \
    sh -c 'echo "deb [arch=amd64] http://dl.google.com/linux/chrome/deb/ stable main" > /etc/apt/sources.list.d/google-chrome.list' && \
    apt-get update && apt-get install -y \
    google-chrome-stable \
    && rm -rf /var/lib/apt/lists/*

# Set environment variables
ENV CHROME_BIN=/usr/bin/google-chrome \
    CHROME_PATH=/usr/lib/google-chrome/ \
    XDG_RUNTIME_DIR=/tmp/xdg-runtime-dir

# Create the runtime directory
RUN mkdir -p /tmp/xdg-runtime-dir && chmod 700 /tmp/xdg-runtime-dir

# Expose the remote debugging port
EXPOSE 9222

# Run socat first and then start Chrome as the main process
ENTRYPOINT ["/bin/sh", "-c", "socat TCP-LISTEN:9222,fork TCP:127.0.0.1:9223 & sleep 2 && exec /usr/bin/google-chrome --headless --no-sandbox --disable-gpu --disable-software-rasterizer --disable-dev-shm-usage --no-zygote --disable-extensions --remote-debugging-address=127.0.0.1 --remote-debugging-port=9223"]