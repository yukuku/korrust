import anonymouslibrary.*
import receiverlibrary.checkpoint
fun use(): Single = object : Single { override fun run(value: Int): Int = value }