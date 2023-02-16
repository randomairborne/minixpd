FROM alpine

COPY /executables/minixpd /usr/bin/

ENTRYPOINT "/usr/bin/minixpd"