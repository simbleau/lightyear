# Generates a self-signed certificate valid for 14 days, to use for webtransport
# run this from the root folder
OUT=$(dirname "$0")
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -keyout "$OUT/key.pem" -out "$OUT/cert.pem" -days 14 -nodes -subj "/CN=localhost"
echo "Successfully generated certificate files"
SHA256=$(openssl x509 -in "$OUT/cert.pem" -noout -sha256 -fingerprint | sed 's/^.*=//' | tr -d ':')
echo "Digest: $SHA256"
printf "%s" "$SHA256" > "$OUT/cert.sha256"