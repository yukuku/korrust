package ranked

import java.io.ByteArrayInputStream
import java.io.InputStream

fun chooseChild(x: Child): String = x.choose { "value" }
fun chooseParent(x: Parent): String = x.choose { "value" }
fun read(x: InputStream): Int = x.use { it.read() }
