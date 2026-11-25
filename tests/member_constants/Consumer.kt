package constantconsumer
import constantprobe.Host.Companion.BASE as BASE
import constantprobe.Contract.Companion.LIMIT
import constantprobe.Choice.Companion.CODE
import constantprobe.Settings.LABEL
const val COMBINED = BASE + LIMIT + CODE
const val DESCRIPTION = LABEL + constantprobe.Host.LABEL
fun main() {
    check(COMBINED == 78)
    check(DESCRIPTION == "settingshostsettings")
    check(constantprobe.readAll() == 133)
    println("OK")
}
