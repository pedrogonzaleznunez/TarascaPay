#!/usr/bin/env bash
# Smoke test end-to-end de la API de TarascaPay.
set -euo pipefail

API=http://127.0.0.1:3000/api
PASS=0
FAIL=0

check() { # check <descripcion> <condicion-real> <esperado>
  if [ "$2" = "$3" ]; then
    PASS=$((PASS+1)); printf "  ✅ %s\n" "$1"
  else
    FAIL=$((FAIL+1)); printf "  ❌ %s  (esperado: %s | obtenido: %s)\n" "$1" "$3" "$2"
  fi
}

STAMP=$(date +%s)
echo "== 1. Registro de usuarios =="
ANA=$(curl -s -X POST $API/auth/register -H 'Content-Type: application/json' \
  -d "{\"email\":\"ana+$STAMP@test.com\",\"password\":\"clave12345\",\"display_name\":\"Ana\",\"currency\":\"ARS\"}")
BETO=$(curl -s -X POST $API/auth/register -H 'Content-Type: application/json' \
  -d "{\"email\":\"beto+$STAMP@test.com\",\"password\":\"clave12345\",\"display_name\":\"Beto\"}")
CARO=$(curl -s -X POST $API/auth/register -H 'Content-Type: application/json' \
  -d "{\"email\":\"caro+$STAMP@test.com\",\"password\":\"clave12345\",\"display_name\":\"Caro\"}")

TA=$(echo "$ANA" | jq -r .token); IA=$(echo "$ANA" | jq -r .user.id)
TB=$(echo "$BETO" | jq -r .token); IB=$(echo "$BETO" | jq -r .user.id)
TC=$(echo "$CARO" | jq -r .token); IC=$(echo "$CARO" | jq -r .user.id)
check "3 usuarios creados con token" "$([ -n "$TA" ] && [ -n "$TB" ] && [ -n "$TC" ] && echo ok)" "ok"

echo "== 2. Validaciones de registro/login =="
DUP=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/auth/register -H 'Content-Type: application/json' \
  -d "{\"email\":\"ana+$STAMP@test.com\",\"password\":\"clave12345\",\"display_name\":\"Otra Ana\"}")
check "email duplicado rechazado" "$DUP" "409"

WEAK=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/auth/register -H 'Content-Type: application/json' \
  -d '{"email":"x@test.com","password":"corta","display_name":"X"}')
check "contraseña corta rechazada" "$WEAK" "400"

BADMAIL=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/auth/register -H 'Content-Type: application/json' \
  -d '{"email":"noesunmail","password":"clave12345","display_name":"X"}')
check "email inválido rechazado" "$BADMAIL" "400"

BADLOGIN=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/auth/login -H 'Content-Type: application/json' \
  -d "{\"email\":\"ana+$STAMP@test.com\",\"password\":\"incorrecta\"}")
check "login con clave incorrecta rechazado" "$BADLOGIN" "401"

OKLOGIN=$(curl -s -X POST $API/auth/login -H 'Content-Type: application/json' \
  -d "{\"email\":\"ana+$STAMP@test.com\",\"password\":\"clave12345\"}" | jq -r .user.display_name)
check "login correcto" "$OKLOGIN" "Ana"

NOAUTH=$(curl -s -o /dev/null -w '%{http_code}' $API/groups)
check "sin token da 401" "$NOAUTH" "401"

BADTOK=$(curl -s -o /dev/null -w '%{http_code}' $API/groups -H 'Authorization: Bearer basura')
check "token inválido da 401" "$BADTOK" "401"

echo "== 3. Grupo de viaje =="
G=$(curl -s -X POST $API/groups -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d '{"name":"Viaje a Bariloche","kind":"trip","currency":"ARS","description":"Finde largo"}')
GID=$(echo "$G" | jq -r .id)
CODE=$(echo "$G" | jq -r .invite_code)
check "grupo creado" "$(echo "$G" | jq -r .name)" "Viaje a Bariloche"
check "creador es owner" "$(echo "$G" | jq -r .role)" "owner"
check "emoji por defecto de viaje" "$(echo "$G" | jq -r .emoji)" "✈️"

curl -s -X POST $API/groups/join -H "Authorization: Bearer $TB" -H 'Content-Type: application/json' \
  -d "{\"invite_code\":\"$CODE\"}" > /dev/null
M=$(curl -s -X POST $API/groups/$GID/members -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d "{\"email\":\"caro+$STAMP@test.com\"}")
check "3 miembros en el grupo" "$(echo "$M" | jq 'length')" "3"

OUTSIDER=$(curl -s -o /dev/null -w '%{http_code}' $API/groups/$GID -H "Authorization: Bearer $TA")
check "miembro accede al grupo" "$OUTSIDER" "200"

echo "== 4. Gastos con distintos repartos =="
# Ana paga 30000 (300.00) en partes iguales entre 3 => 100.00 c/u
E1=$(curl -s -X POST $API/expenses -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Cabaña\",\"amount_cents\":30000,\"split_type\":\"equal\"}")
E1ID=$(echo "$E1" | jq -r .id)
check "gasto equal: 3 partes" "$(echo "$E1" | jq '.splits | length')" "3"
check "gasto equal: partes suman el total" "$(echo "$E1" | jq '[.splits[].share_cents] | add')" "30000"
check "gasto equal: cada parte 10000" "$(echo "$E1" | jq '[.splits[].share_cents] | unique | join(",")' -r)" "10000"
check "gasto equal: neto de Ana +20000" "$(echo "$E1" | jq .my_net_cents)" "20000"

# Reparto que no divide exacto: 10.00 entre 3
E2=$(curl -s -X POST $API/expenses -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Peaje\",\"amount_cents\":1000,\"split_type\":\"equal\"}")
check "reparto no exacto suma el total (sin perder centavos)" \
  "$(echo "$E2" | jq '[.splits[].share_cents] | add')" "1000"

# Beto paga 15000 con porcentajes 50/30/20
E3=$(curl -s -X POST $API/expenses -H "Authorization: Bearer $TB" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Cena\",\"amount_cents\":15000,\"split_type\":\"percentage\",\"splits\":[{\"user_id\":\"$IA\",\"value\":5000},{\"user_id\":\"$IB\",\"value\":3000},{\"user_id\":\"$IC\",\"value\":2000}]}")
check "gasto percentage creado" "$(echo "$E3" | jq -r .split_type)" "percentage"
check "percentage: parte de Ana = 50%" \
  "$(echo "$E3" | jq --arg u "$IA" '[.splits[] | select(.user.id==$u) | .share_cents][0]')" "7500"

BADPCT=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/expenses -H "Authorization: Bearer $TB" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Mal\",\"amount_cents\":1000,\"split_type\":\"percentage\",\"splits\":[{\"user_id\":\"$IA\",\"value\":5000},{\"user_id\":\"$IB\",\"value\":3000}]}")
check "porcentajes que no suman 100% se rechazan" "$BADPCT" "400"

BADEXACT=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/expenses -H "Authorization: Bearer $TB" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Mal\",\"amount_cents\":1000,\"split_type\":\"exact\",\"splits\":[{\"user_id\":\"$IA\",\"value\":400},{\"user_id\":\"$IB\",\"value\":300}]}")
check "importes exactos que no cuadran se rechazan" "$BADEXACT" "400"

NEG=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/expenses -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Negativo\",\"amount_cents\":-500}")
check "importe negativo rechazado" "$NEG" "400"

# Caro paga 6000 por partes (shares 2:1:1)
E4=$(curl -s -X POST $API/expenses -H "Authorization: Bearer $TC" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Nafta\",\"amount_cents\":6000,\"split_type\":\"shares\",\"splits\":[{\"user_id\":\"$IA\",\"value\":2},{\"user_id\":\"$IB\",\"value\":1},{\"user_id\":\"$IC\",\"value\":1}]}")
check "shares: Ana paga el doble" \
  "$(echo "$E4" | jq --arg u "$IA" '[.splits[] | select(.user.id==$u) | .share_cents][0]')" "3000"

echo "== 5. Permisos =="
DENY=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/expenses -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Ajeno\",\"amount_cents\":100,\"split_type\":\"equal\",\"splits\":[{\"user_id\":\"00000000-0000-0000-0000-000000000001\",\"value\":0}]}")
check "participante fuera del grupo rechazado" "$DENY" "400"

# Un cuarto usuario que no pertenece al grupo
DANI=$(curl -s -X POST $API/auth/register -H 'Content-Type: application/json' \
  -d "{\"email\":\"dani+$STAMP@test.com\",\"password\":\"clave12345\",\"display_name\":\"Dani\"}")
TD=$(echo "$DANI" | jq -r .token)
NOMEMBER=$(curl -s -o /dev/null -w '%{http_code}' $API/groups/$GID -H "Authorization: Bearer $TD")
check "no-miembro no ve el grupo" "$NOMEMBER" "403"
NOEXP=$(curl -s -o /dev/null -w '%{http_code}' $API/expenses/$E1ID -H "Authorization: Bearer $TD")
check "no-miembro no ve el gasto" "$NOEXP" "403"

echo "== 6. Balances y simplificación =="
B=$(curl -s $API/groups/$GID/balances -H "Authorization: Bearer $TA")
SUMNET=$(echo "$B" | jq '[.balances[].net_cents] | add')
check "los saldos netos suman cero" "$SUMNET" "0"
check "total gastado del grupo" "$(echo "$B" | jq .total_spent_cents)" "52000"
NTRANSF=$(echo "$B" | jq '.transfers | length')
check "transferencias sugeridas <= n-1" "$([ "$NTRANSF" -le 2 ] && echo ok)" "ok"

echo "== 7. Saldar deuda =="
# Beto le paga a Ana lo que la simplificación indique (si aplica)
OWE=$(echo "$B" | jq --arg u "$IB" '[.transfers[] | select(.from.id==$u) | .amount_cents] | add // 0')
if [ "$OWE" != "0" ] && [ "$OWE" != "null" ]; then
  TO=$(echo "$B" | jq -r --arg u "$IB" '[.transfers[] | select(.from.id==$u) | .to.id][0]')
  curl -s -X POST $API/groups/$GID/settlements -H "Authorization: Bearer $TB" -H 'Content-Type: application/json' \
    -d "{\"from_user\":\"$IB\",\"to_user\":\"$TO\",\"amount_cents\":$OWE,\"note\":\"transferencia\"}" > /dev/null
  B2=$(curl -s $API/groups/$GID/balances -H "Authorization: Bearer $TA")
  NEWNET=$(echo "$B2" | jq --arg u "$IB" '[.balances[] | select(.user.id==$u) | .net_cents][0]')
  check "tras saldar, el saldo de Beto queda en 0" "$NEWNET" "0"
  check "los saldos siguen sumando cero" "$(echo "$B2" | jq '[.balances[].net_cents] | add')" "0"
fi

SELFPAY=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/groups/$GID/settlements -H "Authorization: Bearer $TB" -H 'Content-Type: application/json' \
  -d "{\"from_user\":\"$IB\",\"to_user\":\"$IB\",\"amount_cents\":100}")
check "pago a uno mismo rechazado" "$SELFPAY" "400"

echo "== 8. Gastos personales =="
P=$(curl -s -X POST $API/expenses -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d '{"description":"Café","amount_cents":2500}')
check "gasto personal sin grupo" "$(echo "$P" | jq -r '.group_id // "null"')" "null"
check "gasto personal: una sola parte por el total" "$(echo "$P" | jq '.splits[0].share_cents')" "2500"
PERSONAL=$(curl -s "$API/expenses?personal=true" -H "Authorization: Bearer $TA")
check "listado personal devuelve el gasto" "$(echo "$PERSONAL" | jq 'length')" "1"
PRIV=$(curl -s -o /dev/null -w '%{http_code}' $API/expenses/$(echo "$P" | jq -r .id) -H "Authorization: Bearer $TB")
check "gasto personal ajeno es privado" "$PRIV" "403"

echo "== 9. Comprobantes =="
# PNG mínimo válido (1x1)
printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89' > /tmp/tp_test.png
UP=$(curl -s -X POST $API/attachments -H "Authorization: Bearer $TA" -F "file=@/tmp/tp_test.png")
AID=$(echo "$UP" | jq -r .id)
check "adjunto subido y detectado como PNG" "$(echo "$UP" | jq -r .mime_type)" "image/png"

echo "no soy una imagen" > /tmp/tp_fake.png
BADUP=$(curl -s -o /dev/null -w '%{http_code}' -X POST $API/attachments -H "Authorization: Bearer $TA" -F "file=@/tmp/tp_fake.png")
check "archivo disfrazado de PNG rechazado" "$BADUP" "400"

E5=$(curl -s -X POST $API/expenses -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Super\",\"amount_cents\":8000,\"attachment_ids\":[\"$AID\"]}")
check "gasto con comprobante vinculado" "$(echo "$E5" | jq '.attachments | length')" "1"

FILEOK=$(curl -s -o /dev/null -w '%{http_code}' "$API/attachments/$AID/file" -H "Authorization: Bearer $TB")
check "otro miembro del grupo ve el comprobante" "$FILEOK" "200"
FILEDENY=$(curl -s -o /dev/null -w '%{http_code}' "$API/attachments/$AID/file" -H "Authorization: Bearer $TD")
check "ajeno al grupo no ve el comprobante" "$FILEDENY" "403"
FILETOKEN=$(curl -s -o /dev/null -w '%{http_code}' "$API/attachments/$AID/file?token=$TA")
check "token por query string funciona (para <img>)" "$FILETOKEN" "200"

echo "== 10. Edición y borrado =="
UPD=$(curl -s -X PATCH $API/expenses/$E1ID -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d '{"amount_cents":60000}')
check "al editar el importe se recalculan las partes" "$(echo "$UPD" | jq '[.splits[].share_cents] | add')" "60000"

CM=$(curl -s -X POST $API/expenses/$E1ID/comments -H "Authorization: Bearer $TB" -H 'Content-Type: application/json' \
  -d '{"body":"¿Incluye el desayuno?"}')
check "comentario agregado" "$(echo "$CM" | jq -r .user.display_name)" "Beto"

DEL=$(curl -s -o /dev/null -w '%{http_code}' -X DELETE $API/expenses/$E1ID -H "Authorization: Bearer $TA")
check "gasto eliminado" "$DEL" "200"
GONE=$(curl -s -o /dev/null -w '%{http_code}' $API/expenses/$E1ID -H "Authorization: Bearer $TA")
check "el gasto borrado ya no aparece" "$GONE" "404"

echo "== 11. Panel y estadísticas =="
D=$(curl -s $API/dashboard -H "Authorization: Bearer $TA")
check "dashboard responde con moneda" "$(echo "$D" | jq -r .currency)" "ARS"
check "dashboard cuenta el grupo activo" "$(echo "$D" | jq .active_groups)" "1"
S=$(curl -s "$API/stats?group_id=$GID" -H "Authorization: Bearer $TA")
check "estadísticas por categoría" "$([ "$(echo "$S" | jq '.by_category | length')" -ge 1 ] && echo ok)" "ok"
CATS=$(curl -s $API/categories -H "Authorization: Bearer $TA" | jq 'length')
check "13 categorías precargadas" "$CATS" "13"
ACT=$(curl -s $API/groups/$GID/activity -H "Authorization: Bearer $TA" | jq 'length')
check "historial de actividad registrado" "$([ "$ACT" -ge 3 ] && echo ok)" "ok"

echo "== 12. Perfil =="
PR=$(curl -s -X PUT $API/auth/profile -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d '{"display_name":"Ana María","currency":"USD"}')
check "perfil actualizado" "$(echo "$PR" | jq -r .display_name)" "Ana María"
check "moneda actualizada" "$(echo "$PR" | jq -r .currency)" "USD"
PWBAD=$(curl -s -o /dev/null -w '%{http_code}' -X PUT $API/auth/password -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d '{"current_password":"equivocada","new_password":"nuevaclave123"}')
check "cambio de clave con clave actual incorrecta rechazado" "$PWBAD" "401"

echo "== 13. Edición: notas y comprobantes ajenos =="
# Ana carga un gasto que pagó Beto, con un comprobante subido por ella.
UP2=$(curl -s -X POST $API/attachments -H "Authorization: Bearer $TA" -F "file=@/tmp/tp_test.png")
AID2=$(echo "$UP2" | jq -r .id)
E6=$(curl -s -X POST $API/expenses -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d "{\"group_id\":\"$GID\",\"description\":\"Entradas\",\"amount_cents\":4000,\"paid_by\":\"$IB\",\"notes\":\"pagar antes del viernes\",\"attachment_ids\":[\"$AID2\"]}")
E6ID=$(echo "$E6" | jq -r .id)
check "gasto de Ana pagado por Beto, con comprobante" "$(echo "$E6" | jq '.attachments | length')" "1"

# Beto lo edita (puede porque él pagó) sin tocar el comprobante de Ana.
E6B=$(curl -s -X PATCH $API/expenses/$E6ID -H "Authorization: Bearer $TB" -H 'Content-Type: application/json' \
  -d "{\"amount_cents\":5000,\"attachment_ids\":[\"$AID2\"]}")
check "el comprobante ajeno sobrevive a la edición" "$(echo "$E6B" | jq '.attachments | length')" "1"

# La nota se puede borrar mandando una cadena vacía.
E6C=$(curl -s -X PATCH $API/expenses/$E6ID -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d '{"notes":""}')
check "la nota se puede borrar" "$(echo "$E6C" | jq -r '.notes // "null"')" "null"

# Omitir el campo no la toca.
curl -s -X PATCH $API/expenses/$E6ID -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d '{"notes":"nota nueva"}' > /dev/null
E6D=$(curl -s -X PATCH $API/expenses/$E6ID -H "Authorization: Bearer $TA" -H 'Content-Type: application/json' \
  -d '{"description":"Entradas al teatro"}')
check "omitir la nota la conserva" "$(echo "$E6D" | jq -r .notes)" "nota nueva"

# Quitarlo de la lista sí lo desvincula.
E6E=$(curl -s -X PATCH $API/expenses/$E6ID -H "Authorization: Bearer $TB" -H 'Content-Type: application/json' \
  -d '{"attachment_ids":[]}')
check "sacar el comprobante de la lista lo desvincula" "$(echo "$E6E" | jq '.attachments | length')" "0"

echo
echo "════════════════════════════════════"
printf "  ✅ %d correctas   ❌ %d fallidas\n" "$PASS" "$FAIL"
echo "════════════════════════════════════"
[ "$FAIL" -eq 0 ]
