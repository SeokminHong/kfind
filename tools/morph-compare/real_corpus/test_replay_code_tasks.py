import unittest

from replay_code_tasks import verify_preserved, verify_threads


class CodeTaskValidationTests(unittest.TestCase):
    def test_repair_cannot_replace_or_remove_original_tests(self):
        original = (
            "fn value() { 0 }\n#[cfg(test)]\nmod tests { assert_eq!(value(), 1); }"
        )
        verify_preserved(
            original, original.replace("fn value() { 0 }", "fn value() { 1 }")
        )
        for changed in [
            original.replace("assert_eq!(value(), 1);", ""),
            "fn value() { 1 }",
        ]:
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                verify_preserved(original, changed)

    def test_thread_repair_cannot_weaken_main_validation(self):
        original = "collect();\n    if results.len() != 10 { panic!(); }"
        verify_preserved(original, original.replace("collect();", "join_all();"))
        with self.assertRaises(ValueError):
            verify_preserved(original, original.replace("!= 10", "!= 0"))

    def test_worker_identity_and_duration_must_match_the_task(self):
        output = "\n".join(
            [f"Thread {i} done" for i in range(10)]
            + [f"Thread {i} took 250ms" for i in range(10)]
        )
        verify_threads(output)
        for changed in [
            output.replace("Thread 0 done", "Thread 1 done"),
            output.replace("250ms", "249ms"),
            output.replace("Thread 0 took 250ms", ""),
        ]:
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                verify_threads(changed)


if __name__ == "__main__":
    unittest.main()
