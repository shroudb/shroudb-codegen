// ShrouDB Auth Go HTTP client integration test.
package main

import (
	"context"
	"fmt"
	"net/http"
	"os"

	shroudb_auth "github.com/shroudb/shroudb-auth-go"
)

var passed, failed int

func check(name string, condition bool) {
	if condition {
		passed++
		fmt.Printf("  PASS  %s\n", name)
	} else {
		failed++
		fmt.Printf("  FAIL  %s\n", name)
	}
}

// contentTypeTransport ensures Content-Type: application/json is set on all POST requests.
// The server requires it even for bodyless POST endpoints (refresh, logout).
type contentTypeTransport struct {
	base http.RoundTripper
}

func (t *contentTypeTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	if req.Method == "POST" && req.Header.Get("Content-Type") == "" {
		req.Header.Set("Content-Type", "application/json")
	}
	return t.base.RoundTrip(req)
}

func main() {
	baseURL := os.Getenv("SHROUDB_AUTH_TEST_URL")
	if baseURL == "" {
		baseURL = "http://127.0.0.1:4001"
	}

	httpClient := &http.Client{
		Transport: &contentTypeTransport{base: http.DefaultTransport},
	}

	client := shroudb_auth.NewClient(
		baseURL,
		shroudb_auth.WithKeyspace("default"),
		shroudb_auth.WithHTTPClient(httpClient),
	)
	ctx := context.Background()

	// 1. Health
	h, err := client.Health(ctx)
	check("health", err == nil && (h.Status == "healthy" || h.Status == "ok" || h.Status == "OK"))

	// 2. Signup
	signup, err := client.Signup(ctx, "testpass123", "testuser_go", nil)
	check("signup", err == nil && signup.AccessToken != "" && signup.RefreshToken != "")
	var accessToken, refreshToken string
	if err == nil {
		accessToken = signup.AccessToken
		refreshToken = signup.RefreshToken
	}

	// 3. Session (verify access token)
	client.AccessToken = accessToken
	session, err := client.Session(ctx)
	check("session", err == nil && session.UserId != nil && *session.UserId == "testuser_go")

	// 4. Login
	login, err := client.Login(ctx, "testpass123", "testuser_go")
	check("login", err == nil && login.AccessToken != "")

	// 5. Refresh
	client.RefreshToken = refreshToken
	ref, err := client.Refresh(ctx)
	check("refresh", err == nil && ref.AccessToken != "")

	// 6. Change password
	client.AccessToken = login.AccessToken
	_, err = client.ChangePassword(ctx, "newpass456", "testpass123")
	check("change_password", err == nil)

	// 7. Login with new password
	login2, err := client.Login(ctx, "newpass456", "testuser_go")
	check("login_new_pass", err == nil && login2.AccessToken != "")

	// 8. Forgot password
	fp, err := client.ForgotPassword(ctx, "testuser_go")
	check("forgot_password", err == nil && fp.ResetToken != nil && *fp.ResetToken != "")

	// 9. Reset password
	var resetToken string
	if fp != nil && fp.ResetToken != nil {
		resetToken = *fp.ResetToken
	}
	_, err = client.ResetPassword(ctx, "resetpass789", resetToken)
	check("reset_password", err == nil)

	// 10. Login after reset
	login3, err := client.Login(ctx, "resetpass789", "testuser_go")
	check("login_after_reset", err == nil && login3.AccessToken != "")

	// 11. Logout
	if login3 != nil {
		client.AccessToken = login3.AccessToken
		client.RefreshToken = login3.RefreshToken
	}
	_, err = client.Logout(ctx)
	check("logout", err == nil)

	// 12. JWKS
	jwks, err := client.Jwks(ctx)
	check("jwks", err == nil && jwks.Keys != nil)

	// 13. Error: wrong password
	_, err = client.Login(ctx, "wrongpass", "testuser_go")
	check("error_unauthorized", err != nil)

	// 14. Error: duplicate signup
	_, err = client.Signup(ctx, "anotherpass", "testuser_go", nil)
	check("error_conflict", err != nil)

	fmt.Printf("\n%d passed, %d failed\n", passed, failed)
	if failed > 0 {
		os.Exit(1)
	}
}
