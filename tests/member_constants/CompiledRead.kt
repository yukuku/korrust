package compiledconstantprobe
import constantprobe.Host
import constantprobe.Host.Companion.BASE as IMPORTED
import constantprobe.Contract.Companion.LIMIT
import constantprobe.Choice.Companion.CODE
const val TOTAL = Host.BASE + Host.Companion.BASE + IMPORTED + LIMIT + CODE
fun read(): Int = Host.BASE + Host.Companion.BASE + IMPORTED + LIMIT + CODE
