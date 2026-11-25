package bodyconsumer
import interfacebodies.*
fun consume(): String {
    val generic: Generic<String> = Text()
    val ordinary: Ordinary = Plain()
    val named: Name = generic.bounded(Name("K"))
    val direct: String = Text().identity("L")
    val inherited: String = RefinedImpl().relay("M")
    val nullable: String? = generic.identity(null)
    return generic.echo("N") + ordinary.identity("O") + named.text() + direct + inherited + if (nullable == null) { "P" } else { "bad" }
}
