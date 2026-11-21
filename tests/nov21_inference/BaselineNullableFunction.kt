package baseline
import inference.dependency.mapMaybe
fun transform(value: Int): String? = mapMaybe(value) { number -> number.toString() }
