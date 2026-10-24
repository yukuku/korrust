package verification.serialization

import example.routes.deep.EmptyRoute
import independent.navigation.AliasedDestination
import unrelated.tokens.QualifiedToken
import kotlinx.serialization.json.Json
import kotlinx.serialization.serializer

fun main() {
    check(EmptyRoute.serializer() === serializer<EmptyRoute>())
    check(AliasedDestination.serializer() === serializer<AliasedDestination>())
    check(QualifiedToken.serializer() === serializer<QualifiedToken>())
    check(EmptyRoute.serializer().descriptor.serialName == "example.routes.deep.EmptyRoute")
    check(AliasedDestination.serializer().descriptor.serialName == "independent.navigation.AliasedDestination")
    check(QualifiedToken.serializer().descriptor.serialName == "unrelated.tokens.QualifiedToken")
    check(Json.encodeToString(EmptyRoute.serializer(), EmptyRoute) == "{}")
    check(Json.decodeFromString(EmptyRoute.serializer(), "{}") === EmptyRoute)
    check(Json.decodeFromString(AliasedDestination.serializer(), "{}") === AliasedDestination)
    check(Json.decodeFromString(QualifiedToken.serializer(), "{}") === QualifiedToken)
    println("{\"id\":\"synthetic_packages_aliases_qualified_names\",\"outcome\":\"pass\",\"checks\":10}")
}
