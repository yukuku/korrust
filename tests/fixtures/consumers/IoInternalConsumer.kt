package korrust.consumers
import korrust.probes.io.HiddenBuffers
fun main() { check(HiddenBuffers.size() == 3); println("internal object friend consumer OK") }
