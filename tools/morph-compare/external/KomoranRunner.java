import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.StringJoiner;

import kr.co.shineware.nlp.komoran.constant.DEFAULT_MODEL;
import kr.co.shineware.nlp.komoran.core.Komoran;
import kr.co.shineware.nlp.komoran.model.Token;

public final class KomoranRunner {
    private KomoranRunner() {}

    public static void main(String[] args) throws Exception {
        long initializationStarted = System.nanoTime();
        Komoran komoran = new Komoran(DEFAULT_MODEL.FULL);
        long initializationNanos = System.nanoTime() - initializationStarted;
        BufferedReader input = new BufferedReader(
            new InputStreamReader(System.in, StandardCharsets.UTF_8)
        );
        Base64.Decoder decoder = Base64.getDecoder();
        Base64.Encoder encoder = Base64.getEncoder();
        List<Long> latencies = new ArrayList<>();
        long evaluationNanos = 0;
        String line;
        while ((line = input.readLine()) != null) {
            String text = new String(decoder.decode(line), StandardCharsets.UTF_8);
            StringJoiner tokens = new StringJoiner(";");
            long caseStarted = System.nanoTime();
            for (Token token : komoran.analyze(text).getTokenList()) {
                String encodedMorph = encoder.encodeToString(
                    token.getMorph().getBytes(StandardCharsets.UTF_8)
                );
                tokens.add(
                    encodedMorph + "," + token.getPos()
                        + "," + token.getBeginIndex()
                        + "," + token.getEndIndex()
                );
            }
            long latency = System.nanoTime() - caseStarted;
            latencies.add(latency);
            evaluationNanos += latency;
            System.out.println(tokens);
        }
        StringJoiner encodedLatencies = new StringJoiner(",");
        for (long latency : latencies) {
            encodedLatencies.add(Long.toString(latency));
        }
        System.err.println(
            "KFIND_PERF\t" + initializationNanos
                + "\t" + evaluationNanos
                + "\t" + encodedLatencies
        );
    }
}
