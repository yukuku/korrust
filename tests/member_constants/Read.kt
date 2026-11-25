package constantprobe
import constantprobe.Host.Companion.BASE as IMPORTED
const val FROM_COMPANION = Host.Companion.BASE
const val FROM_IMPORT = IMPORTED + Contract.LIMIT
fun readAll(): Int = Host.BASE + Host.Companion.BASE + Contract.LIMIT + Choice.CODE + Settings.TOTAL
