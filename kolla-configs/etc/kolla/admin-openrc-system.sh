# Ansible managed

# Clear any old environment that may conflict.
for key in $( set | awk '{FS="="}  /^OS_/ {print $1}' ); do unset $key ; done
export OS_USER_DOMAIN_NAME='Default'
export OS_SYSTEM_SCOPE=all
export OS_USERNAME='admin'
export OS_PASSWORD='8FKbMLJ0YA7S2f90PeA89hiCWTdfsLYW1BmpFJmO'
export OS_AUTH_URL='http://10.171.79.100:5000'
export OS_INTERFACE='internal'
export OS_ENDPOINT_TYPE='internalURL'
export OS_IDENTITY_API_VERSION='3'
export OS_REGION_NAME='RegionOne'
export OS_AUTH_PLUGIN='password'
