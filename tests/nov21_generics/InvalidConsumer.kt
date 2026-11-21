package consumers
import methodgenerics.Generic
fun bad(generic: Generic<String>): String = generic.bounded("wrong")
