package nested.consumer

import nested.library.`Dollar$Pretend` as LegalDollar
import nested.library.Outer.Item as Token
import nested.library.Outer.Kind
import nested.library.Outer.Box
import nested.library.Outer.Inner
import nested.library.Outer.Middle.Leaf
import nested.library.Outer.Middle.Leaf.text as leafText
import nested.library.KotlinOuter.Nested as KotlinToken

class Outer
fun value(): Int = Token(5).value
fun kind(): Kind = Kind.B
fun leaf(): String = Leaf.text() + ":" + leafText()
fun unbox(box: Box<String>): String = box.get()
fun inner(value: Inner): Int = value.number()
fun shadow(Token: () -> String): String = Token()
fun kotlinNested(): Int = KotlinToken(4).value

fun dollar(): Int = LegalDollar.number()
