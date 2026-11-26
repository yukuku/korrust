package anonymousconsumer
import anonymousfunctions.make
import anonymousfunctions.generic
import kotlinx.coroutines.runBlocking
fun consumer(): Int = runBlocking { make(2).first(1) }
fun consumerGeneric(): String = runBlocking { generic().echo("k") }
fun consumerSam(): Int = runBlocking { anonymousfunctions.sam(1).apply(2) }
fun staticValue(): String = anonymousfunctions.narrow()
