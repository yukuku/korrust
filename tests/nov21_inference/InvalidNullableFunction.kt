package invalid
import inference.dependency.mapMaybe
fun bad(): String? = mapMaybe(1) { value: String -> value }
