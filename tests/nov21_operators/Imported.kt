package updates
import foreign.Meter
import foreign.Bucket
import foreign.Switch
import foreign.inc
import foreign.dec
fun compiled(value: Bucket): Int { val previous = value[7L]++; value[7L] += 2; return previous.value * 10 + value[7L].value }
fun compiledExtension(): Int { var value = Switch(4); val old = value++; value--; return old.value * 10 + value.value }
