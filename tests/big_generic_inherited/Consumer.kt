import inheriteddefaults.*
class ConsumerChild:Parent<String>("seed") { override fun <R> choose(value:R,suffix:String):String="consumer:"+suffix }
fun consume():String {
 val child=ConsumerChild()
 val parent:Parent<String> = child
 return child.choose(1)+parent.choose(1)+TextChild().choose(1)+GrandChild().choose(1)+IntChild().keep(7,"kept")+TextChild().supply(1)
}
