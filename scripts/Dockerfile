FROM frolvlad/alpine-glibc
# FROM fantablade/libssl
#VOLUME [ "/data" ]
#RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories && apk update && apk add openssl 
COPY ./together /opt/together
COPY ./static /opt/static/
CMD ["/opt/together"]
