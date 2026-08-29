import { Router, Routes, Route } from '@solidjs/router';
import { AuthProvider } from './contexts/AuthContext';
import { ProtectedRoute } from './components/auth/ProtectedRoute';
import { SignIn } from './components/auth/SignIn';
import { SignUp } from './components/auth/SignUp';
import { ForgotPassword } from './components/auth/ForgotPassword';
import { ResetPassword } from './components/auth/ResetPassword';
import { Dashboard } from './pages/Dashboard';
import { Home } from './pages/Home';
import { ListingList } from './components/marketplace/ListingList';
import { ListingCreate } from './components/marketplace/ListingCreate';
import { ListingEdit } from './components/marketplace/ListingEdit';

function App() {
  return (
    <AuthProvider>
      <Router>
        <Routes>
          {/* Public routes */}
          <Route path="/" element={<Home />} />
          <Route path="/sign-in" element={<SignIn />} />
          <Route path="/sign-up" element={<SignUp />} />
          <Route path="/forgot-password" element={<ForgotPassword />} />
          <Route path="/reset-password" element={<ResetPassword />} />
          
          {/* Marketplace listings are publicly browsable */}
          <Route path="/listings" element={<ListingList />} />

          {/* Protected routes */}
          <Route path="/dashboard" element={
            <ProtectedRoute>
              <Dashboard />
            </ProtectedRoute>
          } />
          <Route path="/listings/new" element={
            <ProtectedRoute>
              <ListingCreate />
            </ProtectedRoute>
          } />
          <Route path="/listings/:id/edit" element={
            <ProtectedRoute>
              <ListingEdit />
            </ProtectedRoute>
          } />

          {/* Add more protected routes as needed */}
          {/* <Route path="/profile" element={
            <ProtectedRoute>
              <Profile />
            </ProtectedRoute>
          } /> */}
        </Routes>
      </Router>
    </AuthProvider>
  );
}

export default App;