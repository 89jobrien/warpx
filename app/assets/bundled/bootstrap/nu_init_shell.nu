# Nushell init shell script — sends the InitShell DCS hook to Warp.
# Executed via `nu -e` from a bash wrapper; must be valid Nushell syntax.
# IMPORTANT: no single quotes allowed — this script is embedded in bash single quotes.
let warp_session_id = (random int 0..999999999)
let _hostname = (try { hostname | str trim } catch { (sys host).hostname })
let _user = (try { whoami | str trim } catch { $env.USER? | default "" })
let _json = $"{\"hook\": \"InitShell\", \"value\": {\"session_id\": ($warp_session_id), \"shell\": \"nu\", \"user\": \"($_user)\", \"hostname\": \"($_hostname)\"}}"
let _msg = ($_json | into binary | encode hex | str downcase)
let _dcs = (char -i 0x1b) + (char -i 0x50) + (char -i 0x24) + "d" + $_msg + (char -i 0x9c)
print -n $_dcs
$env.WARP_SESSION_ID = ($warp_session_id | into string)
