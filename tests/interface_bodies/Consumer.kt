package bodyconsumer
import interfacebodies.*
fun consume(): String {
    val generic: Generic<String> = Text()
    val ordinary: Ordinary = Plain()
    val bounded: Name = generic.bounded(Name("K"))
    val direct: String = Text().identity("L")
    val inherited: String = RefinedImpl().relay("M")
    val named: String = Named.identity("object")
    val empty: String = Empty.identity("empty")
    val anonymousResult: String = anonymous().identity("anonymous")
    val nullable: String? = generic.identity(null)
    return generic.echo("N") + ordinary.identity("O") + bounded.text() + direct + inherited + if (nullable == null && named == "object" && empty == "empty" && anonymousResult == "anonymous") { "P" } else { "bad" }
}
