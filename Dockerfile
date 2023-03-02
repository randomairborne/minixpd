FROM alpine

COPY /${TARGETARCH}-executables/minixpd /usr/bin/

ENTRYPOINT "/usr/bin/minixpd"