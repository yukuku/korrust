fun consumer(): String {
    val result: inherited.library.JavaTypes.Base.Result = inherited.consumer.Direct().bodyOnly()
    val token: inherited.library.JavaTypes.Base.Token<String> = inherited.consumer.Direct().token()
    val own: inherited.consumer.Own.Result = inherited.consumer.Own().make()
    return result.tag() + token.value + own.value
}

suspend fun consumerSuspend(): inherited.library.JavaTypes.Base.Result = inherited.consumer.SuspendEnd().execute()

fun parameterExpressions(): String = inherited.consumer.FunctionParameter().staticName<String>() + inherited.consumer.FunctionParameter().genericName<String>() + inherited.consumer.TypeParameter<String>().staticName()

fun genericImportExpression(): String = inherited.explicit.GenericDerived<String>().identity("type-shadow") + inherited.explicit.GenericDerived<String>().staticName()
