package invalid
import inference.dependency.mapMaybe
fun text(value: String): String = value
fun bad(): String? = mapMaybe(1, ::text)
