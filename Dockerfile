FROM debian:stable-slim

COPY stayin-alive /usr/local/bin/stayin-alive

EXPOSE 3000 50051

ENTRYPOINT ["stayin-alive"]
