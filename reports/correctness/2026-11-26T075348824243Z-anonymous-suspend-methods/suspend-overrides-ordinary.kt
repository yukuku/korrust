import anonymouslibrary.*
import receiverlibrary.checkpoint
fun use(): Plain = object : Plain { override suspend fun run(value: Int): Int = value }