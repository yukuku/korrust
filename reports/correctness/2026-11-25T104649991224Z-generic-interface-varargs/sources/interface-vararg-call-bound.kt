interface Label
interface Api<C> { fun <T:Label> first(vararg items:T):T }
fun bad(api:Api<String>):String=api.first("bad")