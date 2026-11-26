public class ConsumerCalls {
    public static void main(String[] args) {
        if (!ConsumerKt.consumer().equals("bodytokenown")) throw new AssertionError();
        if (!ConsumerKt.parameterExpressions().equals("method-paramgeneric:genericclass-param")) throw new AssertionError();
        if (!ConsumerKt.genericImportExpression().equals("type-shadowgeneric-static")) throw new AssertionError();
        System.out.println("consumer-ok");
    }
}
