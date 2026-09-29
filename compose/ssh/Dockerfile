# A throwaway SSH server for the tunnel tests: user tabletist, password
# tabletist, and the test keys from crates/tabletist-db/tests/ssh.
FROM alpine:3.22
RUN apk add --no-cache openssh \
 && ssh-keygen -A \
 && adduser -D -s /bin/sh tabletist \
 && echo 'tabletist:tabletist' | chpasswd \
 && sed -i -e 's/^#\?AllowTcpForwarding.*/AllowTcpForwarding yes/' \
           -e 's/^#\?PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config \
 && printf '%s\n' \
      '# Tests fail logins on purpose and connect in parallel, all from the' \
      '# Docker gateway address: never penalize or throttle that source.' \
      'PerSourcePenalties no' 'MaxStartups 100:30:200' >> /etc/ssh/sshd_config \
 && mkdir -p /home/tabletist/.ssh
COPY crates/tabletist-db/tests/ssh/id_ed25519.pub crates/tabletist-db/tests/ssh/id_ed25519_enc.pub /tmp/keys/
RUN cat /tmp/keys/*.pub > /home/tabletist/.ssh/authorized_keys \
 && chown -R tabletist:tabletist /home/tabletist/.ssh \
 && chmod 700 /home/tabletist/.ssh && chmod 600 /home/tabletist/.ssh/authorized_keys
EXPOSE 22
CMD ["/usr/sbin/sshd", "-D", "-e"]
