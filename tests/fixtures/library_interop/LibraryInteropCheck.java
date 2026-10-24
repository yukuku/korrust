import interop.dep.JavaApi;
import interop.lib.*;
import interop.probes.*;
import java.util.*;

public final class LibraryInteropCheck {
    interface Action { void run() throws Exception; }
    static int checks;
    static void eq(Object actual, Object expected) {
        checks++;
        if (!Objects.equals(actual, expected)) throw new AssertionError("expected " + expected + ", got " + actual);
    }
    static void fails(Class<? extends Throwable> kind, Action action) throws Exception {
        checks++;
        try { action.run(); }
        catch (Throwable error) { if (kind.isInstance(error)) return; throw new AssertionError(kind.getName(), error); }
        throw new AssertionError("missing " + kind.getName());
    }
    public static void main(String[] args) throws Exception {
        String id = args[0];
        switch (id) {
            case "jar_classpath": eq(JarClasspathKt.jarProbe(), 7); break;
            case "kotlin_metadata": eq(KotlinMetadataKt.metadataProbe(), "metadata"); break;
            case "top_level_imports": eq(TopLevelImportsKt.importProbe(), 18); break;
            case "java_generic_signatures": eq(JavaGenericKt.javaGenericProbe(), "signature"); break;
            case "type_variables_bounds": eq(BoundsKt.boundProbe(), 9); break;
            case "generic_inference": eq(InferenceKt.inferenceProbe(), "inferred"); break;
            case "variance_projections":
                eq(VarianceKt.varianceProbe(Arrays.asList("first", "second")), "first");
                fails(IndexOutOfBoundsException.class, () -> VarianceKt.varianceProbe(Collections.emptyList()));
                fails(ClassCastException.class, () -> VarianceKt.varianceProbe((List)Arrays.asList(23)));
                break;
            case "annotations_platform":
                eq(NullabilityKt.platformProbe(false), "platform"); eq(NullabilityKt.nullableProbe(), null); eq(NullabilityKt.annotatedProbe(), 9);
                fails(NullPointerException.class, () -> NullabilityKt.platformProbe(true)); break;
            case "overload_specificity":
                eq(SpecificityKt.specificityProbe(), "string"); eq(SpecificityKt.nullSpecificityProbe(), "string");
                eq(SpecificityKt.nullDefaultSpecificityProbe(), 1); eq(SpecificityKt.genericSpecificityProbe(), 2);
                eq(SpecificityKt.subtypeSpecificityProbe(), 1); break;
            case "compiled_extensions":
                eq(ExtensionsKt.extensionProbe(), 12); eq(ExtensionsKt.stdlibRepeatProbe("ab", 3), "ababab");
                eq(ExtensionsKt.stdlibRepeatProbe("😀", 2), "😀😀"); eq(ExtensionsKt.stdlibRepeatProbe("x", 0), "");
                fails(IllegalArgumentException.class, () -> ExtensionsKt.stdlibRepeatProbe("x", -1)); break;
            case "library_properties":
                LibraryKt.setLibraryMutable(-100); eq(PropertiesKt.propertyProbe(), 49); eq(LibraryKt.getLibraryMutable(), 8);
                eq(PropertiesKt.restrictedGetterProbe(), 6); break;
            case "imported_named_defaults":
                JavaApi.events = ""; eq(DefaultsKt.defaultProbe(), 28); eq(JavaApi.events, "Ba");
                JavaApi.events = ""; eq(DefaultsKt.namedProbe(), 48); eq(JavaApi.events, "BA"); break;
            case "function_invocation":
                eq(FunctionInvocationKt.invocationProbe(value -> value * 3), 27);
                fails(NullPointerException.class, () -> FunctionInvocationKt.invocationProbe(null)); break;
            case "capturing_lambdas":
                eq(CapturesKt.captureProbe(5), 10); eq(CapturesKt.captureProbe(-20), -15); eq(CapturesKt.mutableCaptureProbe(), 9); break;
            case "contextual_lambda_inference": eq(LambdaInferenceKt.lambdaInferenceProbe(), 8); break;
            case "callable_references":
                Counter value = new Counter(10); eq(ReferencesKt.boundReferenceProbe(value), 14); eq(ReferencesKt.unboundReferenceProbe(value), 15);
                fails(NullPointerException.class, () -> ReferencesKt.boundReferenceProbe(null));
                JavaApi.events = ""; eq(ReferencesKt.receiverReferenceProbe(), 23); eq(JavaApi.events, "R"); break;
            case "inherited_dispatch":
                DerivedSource derived = new DerivedSource(); eq(DispatchKt.inheritedProbe(new BaseSource()), 6);
                eq(DispatchKt.inheritedProbe(derived), 12); eq(DispatchKt.interfaceProbe(derived), 12);
                fails(NullPointerException.class, () -> DispatchKt.interfaceProbe(null)); break;
            case "java_sam": eq(SamKt.samProbe(7), 12); eq(SamKt.samProbe(-9), -4); break;
            case "compiled_inline":
                JavaApi.events = ""; eq(InlineKt.inlineProbe(), 17); eq(JavaApi.events, "AB");
                eq(InlineKt.inlineCaptureProbe(5), 16); eq(InlineKt.inlineCaptureProbe(-10), -14);
                eq(InlineKt.inlineMutableCaptureProbe(), 32); eq(InlineKt.inlineBranchProbe(true), 6);
                eq(InlineKt.inlineBranchProbe(false), 3); eq(InlineKt.inlineSimpleProbe(), 18); break;
            case "nonlocal_inline_returns":
                eq(NonlocalKt.nonlocalProbe(), 37); eq(NonlocalKt.nonlocalLongProbe(), 1234567890123L);
                eq(NonlocalKt.nonlocalStringProbe(), "done"); eq(NonlocalKt.nonlocalPrefixProbe(), 37);
                eq(NonlocalKt.nonlocalBranchProbe(true), 37); eq(NonlocalKt.nonlocalBranchProbe(false), -1);
                eq(NonlocalKt.conditionalNonlocalProbe(true), 5L); eq(NonlocalKt.conditionalNonlocalProbe(false), 9L);
                eq(NonlocalKt.nestedNonlocalProbe(), 39L); break;
            default: throw new AssertionError(id);
        }
        System.out.println("{\"id\":\"" + id + "\",\"outcome\":\"pass\",\"checks\":" + checks + "}");
    }
}
