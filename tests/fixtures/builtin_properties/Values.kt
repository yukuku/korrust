package properties

class Measure {
    fun length(): Int = 17
}

fun String.length(): Int = 23
fun property(): Int = "😀".length
fun ordinary(): Int = Measure().length()
fun extension(): Int = "x".length()
