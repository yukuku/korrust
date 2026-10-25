package next.probes
import next.library.stride
import next.library.combine
import next.library.glue
import next.library.Choice
import next.library.pick
import next.library.length

fun descendingCount(start: Int, end: Int): Int { var n = 0; for (i in start downTo end) { n += 1 }; return n }
fun ordinaryDownCount(start: Int, end: Int): Int { val values = start.downTo(end); var n = 0; for (i in values) { n += 1 }; return n }
fun customStrideSum(start: Int, end: Int): Int { var total = 0; for (i in start stride end) { total += i }; return total }
fun sourceInfixInteger(): Int = 3 bump 4
fun sourceMemberInteger(box: InfixBox): Int = box combine 5
fun libraryInfixInteger(): Int = 3 combine 4
fun libraryInfixText(): String = "a" glue "b"
fun memberInfixPriority(): Int = Choice(2) pick 3
fun memberOrdinaryPriority(): Int = Choice(2).pick(3)
fun memberLengthPriority(value: String): Int = value.length
fun sourceOverridesDefault(): String = "prefix".removePrefix("pre")
