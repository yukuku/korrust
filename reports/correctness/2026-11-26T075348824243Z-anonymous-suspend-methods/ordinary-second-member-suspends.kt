import anonymouslibrary.*
import receiverlibrary.checkpoint
fun use(): Single = object : Single { override suspend fun run(value: Int): Int = value; fun extra(): Int = checkpoint() }