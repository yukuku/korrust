package inheriteddefaults
open class Parent<C>(val prefix:C) {
 open fun <T> choose(value:T,suffix:String="default"):String=suffix
 open fun <T> keep(seed:C,value:T,suffix:String="default"):T=value
 open fun <T> supply(value:T,suffix:String="default"):C=prefix
}
open class TextChild:Parent<String>("seed") {
 override fun <R> choose(value:R,suffix:String):String="child:"+suffix
 override fun <R> keep(seed:String,value:R,suffix:String):R=value
 override fun <R> supply(value:R,suffix:String):String="child"
 fun own():String=choose(1)
}
class GrandChild:TextChild() { override fun <S> choose(value:S,suffix:String):String="grand:"+suffix }
class IntChild:Parent<Int>(17) {
 override fun <R> choose(value:R,suffix:String):String="number:"+suffix
 override fun <R> keep(seed:Int,value:R,suffix:String):R=value
 override fun <R> supply(value:R,suffix:String):Int=19
}
fun direct():String = TextChild().choose(1)+TextChild().keep("seed","kept")+TextChild().supply(1)+GrandChild().choose(1)
fun throughBase():String {
 val parent:Parent<String> = TextChild()
 val numbers:Parent<Int> = IntChild()
 return parent.choose(1)+parent.keep("seed","kept")+parent.supply(1)+numbers.choose(1)+numbers.keep(7,"kept")+numbers.supply(1).toString()
}
fun directNumbers():String = IntChild().choose(1)+IntChild().keep(7,"kept")+IntChild().supply(1).toString()
