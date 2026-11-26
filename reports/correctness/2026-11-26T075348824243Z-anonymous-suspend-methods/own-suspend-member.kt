import anonymouslibrary.*
import receiverlibrary.checkpoint
fun use(): Plain = object : Plain { override fun run(value: Int): Int = value; suspend fun extra(): Int = 1 }