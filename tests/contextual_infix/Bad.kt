package consumers
import dep.withValue
import dep.Entry
fun bad(): Entry<String, Int> = "wrong" withValue emptyList()
