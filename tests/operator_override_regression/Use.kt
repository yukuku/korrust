package overrideoperators
interface Accepts<T> { operator fun contains(value:T):Boolean }
class Words:Accepts<String> { override operator fun contains(value:String):Boolean=value=="yes" }
interface Access<T> { operator fun get(key:T):String }
class Lookup:Access<String> { override operator fun get(key:String):String=key }
fun present(n:Int):Boolean=n in 65..90
fun letters(n:Int):Boolean=n in 'A'.code..'Z'.code
fun accepts(value:String):Boolean=value in Words()
fun lookup(value:String):String=Lookup()[value]
