FROM frolvlad/alpine-glibc
# FROM fantablade/libssl
VOLUME [ "/data" ]
#RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.tuna.tsinghua.edu.cn/g' /etc/apk/repositories && apk update && apk add openssl 
COPY ./target/release/together /usr/local/bin/together
CMD ["/usr/local/bin/together"]