package declarations.probes
import declarations.support.Trace
fun literals(): Long = 0xff_ff80ab.toInt().toLong() + 0b1010_0101.toLong() + 1_000.toLong() + 0x1_0000_0000L
fun contextualLong(): Long = 42
