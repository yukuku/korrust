package baseline
import inference.dependency.choose
fun bound(value: String): String? = value.choose(value.length == 0) { null }
