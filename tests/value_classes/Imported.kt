package importedvalue

import valuecases.Id
import valuecases.Text
import valuecases.Wide
import valuecases.createText
import valuecases.createWide
import valuecases.nullableId
import valuecases.nullableText

fun primitive(): Int = Id(7).add(2).raw
fun reference(): String = createText("abc").raw
fun wide(): Long = createWide(9223372036854775807L).raw
fun nullablePrimitive(): Id? = nullableId(null)
fun nullableReference(): Text? = nullableText(null)
