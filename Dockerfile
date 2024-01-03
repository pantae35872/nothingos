# Use the official Ubuntu 20.04 base image
FROM ubuntu:16.04

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive

# Update package lists and install dependencies
RUN apt-get update \
    && apt-get install -y \
        texinfo \
        flex \
        bison \
        python-dev \
        ncurses-dev \
        curl \
	g++	\
	git 	\
	make \
	net-tools \
	rustc \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

ENV PORT=1234

VOLUME /root/env
VOLUME /home
WORKDIR /root/env

EXPOSE 1234
# You can add additional instructions or commands as needed for your specific use case

# CMD ["<your command here>"]  # Uncomment and replace with your command if needed

