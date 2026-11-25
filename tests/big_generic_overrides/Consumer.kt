import genericdispatch.*
fun consume():String {
 val api:Api<String> = TextApi()
 val n:Named = api.bounded(Named("bound"))
 val child:Base<String> = Child()
 return api.echo("echo")+n.text()+api.choose(7)+api.take("seed","take")+child.choose(7)+Singleton.echo("object")+ConsumerImpl().choose(7)
}
class ConsumerImpl:Api<String> {
 override fun <S> echo(value:S):S=value
 override fun <S:Label> bounded(value:S):S=value
 override fun <S> choose(value:S):String="consumer"
 override fun <S> take(seed:String,value:S):S=value
}
