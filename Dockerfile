ARG ARCH=
FROM alpine

COPY /${ARCH}-executables/minixpd /usr/bin/

ENTRYPOINT "/usr/bin/minixpd"