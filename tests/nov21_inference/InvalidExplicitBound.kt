package invalid
import inference.dependency.choose
fun bad(): Int = "value".choose<String, Int>(false) { 1 }
